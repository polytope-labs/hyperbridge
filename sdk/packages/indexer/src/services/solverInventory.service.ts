// Solver inventory and delegation, event-sourced from each chain's own Transfer events.
//
// A solver is tracked on a chain once it fills an order there or the orderbook's watchlist asks
// for it. Tracking starts with one storage read pinned to a block — every supported token's
// balance, every vault's shares, the account's code — and from then on the running totals move
// only with Transfer events. Everything here is written by the chain's own EVM node; the
// Hyperbridge node's only part is queueing watchlist requests (solverWatchlist.service.ts).
//
// The ordering rule that makes the genesis read and the events agree: an EVM block handler runs
// before the block's log handlers, and a read pinned to block N sees N's end state, so it already
// includes every log of N. A row read at N is therefore positioned after all of N's logs, and a
// log is applied only when it comes after the row's position.
import { ethers } from "ethers"

import Erc20Abi from "@/configs/abis/Erc20.abi.json"
import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import {
	SolverDelegation,
	SolverDiscoveryRequest,
	SolverDiscoveryTrigger,
	SolverInventory,
	SolverInventoryHead,
	SolverVaultShares,
	SolverWatchlist,
	SolverWatchlistCursor,
	TrackedSolver,
	TrackedSolverStatus,
} from "@/configs/src/types"
import { SOLVER_ACCOUNT_ADDRESSES } from "@/solver-account-addresses"
import { timestampToDate } from "@/utils/date.helpers"
import { mapConcurrently, readContracts, settle, type Settled, unwrap } from "@/utils/multicall"
import { readAllPages } from "@/utils/store.helpers"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"

const DELEGATION_INDICATOR_PREFIX = "0xef0100"
const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

/** The log position of a storage read: after every log of its block, which the read already includes. */
export const AFTER_EVERY_LOG = 2_147_483_647

/** Genesis reads per block, so a burst of discoveries spreads over blocks instead of stalling one. */
export const MAX_GENESIS_PER_BLOCK = 20
/** Block time between head advances. Well inside the orderbook's 120 s staleness bound. */
export const HEAD_INTERVAL_SECS = 30
/** How often a solver's vault shares are revalued; yield accrues without an event. */
export const REVALUE_INTERVAL_SECS = 3_600
/** How often a solver's balances are re-read against the running totals and its delegation re-checked. */
export const RECONCILE_INTERVAL_SECS = 86_400
/** Tracked solvers examined per head advance, which bounds the refresh's RPC load per block. */
export const REFRESH_PAGE_SIZE = 25
/** Past this many unseen watchlist versions, one scan of the chain's requests beats one query per version. */
const FULL_SCAN_VERSION_GAP = 20

export interface SupportedToken {
	token: string
	vaults: string[]
}

/** The tokens whose inventory is tracked on `chain`, each with the vaults that wrap it; lowercased. */
export function supportedTokens(chain: string): SupportedToken[] {
	return Object.entries(YIELD_VAULT_ADDRESSES[chain] ?? {}).map(([token, vaults]) => ({
		token: token.toLowerCase(),
		vaults: vaults.map((vault) => vault.toLowerCase()),
	}))
}

function tokenForVault(chain: string, vault: string): string | undefined {
	return supportedTokens(chain).find((entry) => entry.vaults.includes(vault))?.token
}

const trackedSolverId = (chain: string, solver: string) => `${chain}-${solver}`
const inventoryId = (chain: string, token: string, solver: string) => `${chain}-${token}-${solver}`
const vaultSharesId = (chain: string, vault: string, solver: string) => `${chain}-${vault}-${solver}`

/** Whether a log at (blockNumber, logIndex) comes after the position a row has been brought up to. */
function isAfter(blockNumber: bigint, logIndex: number, row: { blockNumber: bigint; lastLogIndex: number }): boolean {
	return blockNumber > row.blockNumber || (blockNumber === row.blockNumber && logIndex > row.lastLogIndex)
}

const later = (a: Date, b: Date): Date => (b > a ? b : a)
const elapsedSecs = (from: Date | undefined, to: Date): number =>
	from ? (to.getTime() - from.getTime()) / 1000 : Number.POSITIVE_INFINITY

export interface DelegationReading {
	delegated: boolean
	delegate?: string
}

/** Reads EIP-7702 delegation out of account code: `0xef0100 ‖ addr`, counted only for a known SolverAccount. */
export function parseDelegation(chain: string, code: string | undefined): DelegationReading {
	const lower = (code ?? "0x").toLowerCase()
	if (!lower.startsWith(DELEGATION_INDICATOR_PREFIX) || lower.length !== DELEGATION_INDICATOR_PREFIX.length + 40) {
		return { delegated: false }
	}
	const delegate = `0x${lower.slice(DELEGATION_INDICATOR_PREFIX.length)}`
	const known = (SOLVER_ACCOUNT_ADDRESSES[chain] ?? []).some((account) => account.toLowerCase() === delegate)
	return { delegated: known, delegate }
}

// ─── Discovery ──────────────────────────────────────────────────────────────────────────────────

// The TRACKED solvers of this node's chain. Supported-token Transfers arrive for every holder of
// the token, so the filter that drops the overwhelming majority must cost neither a store read nor
// an RPC. It is complete because a solver only becomes TRACKED through this process; a rollback
// can leave a stale member, which the store read behind it then finds missing.
let trackedCache: { chain: string; solvers: Set<string> } | null = null

async function trackedSolvers(chain: string): Promise<Set<string>> {
	if (trackedCache?.chain === chain) return trackedCache.solvers
	const rows = await readAllPages((limit, offset) =>
		TrackedSolver.getByFields(
			[
				["chain", "=", chain],
				["status", "=", TrackedSolverStatus.TRACKED],
			],
			{ limit, offset, orderBy: "id", orderDirection: "ASC" },
		),
	)
	trackedCache = { chain, solvers: new Set(rows.map((row) => row.solver)) }
	return trackedCache.solvers
}

/** Drops the in-memory tracked set. For tests. */
export function resetSolverInventoryCache(): void {
	trackedCache = null
}

async function queueSolver(
	chain: string,
	solver: string,
	discovery: { discoveredBy: SolverDiscoveryTrigger; discoveredByFill?: string; blockNumber: bigint; at: Date },
): Promise<void> {
	const id = trackedSolverId(chain, solver)
	if (await TrackedSolver.get(id)) return
	await TrackedSolver.create({
		id,
		chain,
		solver,
		status: TrackedSolverStatus.PENDING,
		discoveredBy: discovery.discoveredBy,
		discoveredByFill: discovery.discoveredByFill,
		discoveryBlock: discovery.blockNumber,
		discoveredAt: discovery.at,
	}).save()
}

/**
 * A fill is what makes an address a solver, so the filler starts being tracked. Store-only: the
 * genesis read runs in the next block's handler, whose pinned read already includes this fill and
 * anything else the solver did in this block.
 */
export async function discoverSolverFromFill(params: {
	chain: string
	solver: string
	blockNumber: bigint
	transactionHash: string
	timestamp: bigint
}): Promise<void> {
	if (supportedTokens(params.chain).length === 0) return
	await queueSolver(params.chain, params.solver.toLowerCase(), {
		discoveredBy: SolverDiscoveryTrigger.FILL,
		discoveredByFill: params.transactionHash.toLowerCase(),
		blockNumber: params.blockNumber,
		at: timestampToDate(params.timestamp),
	})
}

/**
 * Turns the watchlist requests the Hyperbridge node queued for this chain into PENDING solvers.
 * The one SolverWatchlist row says whether anything is new, so a quiet block costs two store reads.
 * Both of the Hyperbridge node's entities are read by field query, the rule for every cross-node read.
 */
async function consumeWatchlist(chain: string, blockNumber: bigint, at: Date): Promise<void> {
	// Written by the Hyperbridge node, so read through Postgres: `get` would serve this process's cached copy,
	// which no other node's write invalidates.
	const [watchlist] = await SolverWatchlist.getByFields([["chain", "=", chain]], { limit: 1 })
	if (!watchlist) return
	const cursor = await SolverWatchlistCursor.get(chain)
	const consumed = cursor?.version ?? 0
	if (watchlist.version <= consumed) return

	const requests: SolverDiscoveryRequest[] = []
	if (!cursor || watchlist.version - consumed > FULL_SCAN_VERSION_GAP) {
		requests.push(
			...(await readAllPages((limit, offset) =>
				SolverDiscoveryRequest.getByFields([["chain", "=", chain]], {
					limit,
					offset,
					orderBy: "id",
					orderDirection: "ASC",
				}),
			)),
		)
	} else {
		for (let version = consumed + 1; version <= watchlist.version; version++) {
			requests.push(
				...(await readAllPages((limit, offset) =>
					SolverDiscoveryRequest.getByFields(
						[
							["chain", "=", chain],
							["version", "=", version],
						],
						{ limit, offset, orderBy: "id", orderDirection: "ASC" },
					),
				)),
			)
		}
	}

	for (const request of requests) {
		if (request.version > watchlist.version) continue
		await queueSolver(chain, request.solver, {
			discoveredBy: SolverDiscoveryTrigger.WATCHLIST,
			blockNumber,
			at,
		})
	}
	await SolverWatchlistCursor.create({ id: chain, chain, version: watchlist.version }).save()
}

// ─── Storage reads ──────────────────────────────────────────────────────────────────────────────
//
// Reads are batched across every solver one pass handles: one Multicall3 call carries all of their
// balances, and a second values all of their vault shares. `getCode` has no Multicall3 form, so it
// stays one request per solver — SubQuery hands a mapping a non-batching client on HTTP — sent
// concurrently with the first batch, capped at MAX_CONCURRENT_READS.

const erc20 = new ethers.utils.Interface(Erc20Abi)
const erc4626 = new ethers.utils.Interface(Erc4626Abi)

interface VaultReading {
	vault: string
	shares: bigint
	assets: bigint
	/** On a reconciliation, the read shares minus the tracked ones, valued in assets at the block. */
	drift: bigint
}

interface TokenReading {
	token: string
	wallet: bigint
	vaults: VaultReading[]
}

interface SolverReading extends DelegationReading {
	tokens: TokenReading[]
}

interface ShareAmount {
	vault: string
	shares: bigint
}

/**
 * Values signed share amounts in assets at the handler's block, in one batch. Zero costs no read,
 * and a negative amount is valued by its magnitude: convertToAssets only takes an unsigned one.
 */
async function valueShares(chain: string, amounts: ShareAmount[]): Promise<Settled<bigint>[]> {
	const reads = await readContracts(
		chain,
		amounts
			.filter(({ shares }) => shares !== 0n)
			.map(({ vault, shares }) => ({
				target: vault,
				abi: erc4626,
				method: "convertToAssets",
				args: [(shares < 0n ? -shares : shares).toString()],
			})),
	)
	let next = 0
	return amounts.map(({ shares }): Settled<bigint> => {
		if (shares === 0n) return { ok: true, value: 0n }
		const read = reads[next++]
		if (!read.ok) return read
		const assets = BigInt(read.value[0].toString())
		return { ok: true, value: shares < 0n ? -assets : assets }
	})
}

/** Runs `assemble` for one solver, turning a failed read it unwraps into that solver's error. */
function settleSolver<T>(assemble: () => T): T | Error {
	try {
		return assemble()
	} catch (error) {
		return error instanceof Error ? error : new Error(String(error))
	}
}

interface VaultDraft extends ShareAmount {
	/** Where the shares' valuation sits in the valuation batch. */
	slot: number
	/** Where the drift's valuation sits, when the shares drifted from the tracked position. */
	driftSlot?: number
}

/**
 * Every read tracking needs for each of `solvers`, pinned to the handler's block: every supported
 * token's and vault's `balanceOf` in one batch alongside each solver's `getCode`, then every vault
 * balance valued in a second. A reconciliation also values each vault balance's drift from the
 * tracked position in that second batch. A solver with any failed read comes back as an error, and
 * a failed batch throws.
 */
async function readSolvers(
	chain: string,
	solvers: string[],
	reconciliation: boolean,
): Promise<(SolverReading | Error)[]> {
	const tokens = supportedTokens(chain)
	const holdings = tokens.flatMap(({ token, vaults }) => [token, ...vaults])
	const [balances, codes] = await Promise.all([
		readContracts(
			chain,
			solvers.flatMap((solver) =>
				holdings.map((holding) => ({ target: holding, abi: erc20, method: "balanceOf", args: [solver] })),
			),
		),
		mapConcurrently(solvers, (solver) => settle<string>(`getCode of ${solver}`, (api as any).getCode(solver))),
	])

	// Balances come back solver by solver, each in `holdings` order.
	const amounts: ShareAmount[] = []
	const drafts = solvers.map((_, index) =>
		settleSolver(() => {
			const code = unwrap(codes[index])
			const own = balances
				.slice(index * holdings.length, (index + 1) * holdings.length)
				.map((read) => BigInt(unwrap(read)[0].toString()))
			let next = 0
			return {
				code,
				tokens: tokens.map(({ token, vaults }) => ({
					token,
					wallet: own[next++],
					vaults: vaults.map((vault): VaultDraft => {
						const shares = own[next++]
						return { vault, shares, slot: amounts.push({ vault, shares }) - 1 }
					}),
				})),
			}
		}),
	)
	if (reconciliation) {
		for (const [index, draft] of drafts.entries()) {
			if (draft instanceof Error) continue
			for (const vault of draft.tokens.flatMap((token) => token.vaults)) {
				const position = await SolverVaultShares.get(vaultSharesId(chain, vault.vault, solvers[index]))
				if (position && position.shares !== vault.shares) {
					vault.driftSlot = amounts.push({ vault: vault.vault, shares: vault.shares - position.shares }) - 1
				}
			}
		}
	}

	const values = await valueShares(chain, amounts)
	return drafts.map((draft) =>
		draft instanceof Error
			? draft
			: settleSolver(() => ({
					...parseDelegation(chain, draft.code),
					tokens: draft.tokens.map(({ token, wallet, vaults }) => ({
						token,
						wallet,
						vaults: vaults.map(({ vault, shares, slot, driftSlot }) => ({
							vault,
							shares,
							assets: unwrap(values[slot]),
							drift: driftSlot === undefined ? 0n : unwrap(values[driftSlot]),
						})),
					})),
				})),
	)
}

async function recordDelegation(
	chain: string,
	solver: string,
	reading: DelegationReading,
	blockNumber: bigint,
	at: Date,
): Promise<void> {
	const id = trackedSolverId(chain, solver)
	const existing = await SolverDelegation.get(id)
	const changed =
		!existing || existing.delegated !== reading.delegated || (existing.delegate ?? undefined) !== reading.delegate
	await SolverDelegation.create({
		id,
		chain,
		solver,
		delegated: reading.delegated,
		delegate: reading.delegate,
		blockNumber,
		observedAt: changed || !existing ? at : existing.observedAt,
		refreshedAt: at,
	}).save()
}

/**
 * Writes a storage read over the solver's rows and positions them after the read's block. Used for
 * the genesis read and for each reconciliation; a reconciliation also records how far the running
 * totals had drifted from what the chain holds.
 */
async function applyReading(
	tracked: TrackedSolver,
	reading: SolverReading,
	blockNumber: bigint,
	at: Date,
	reconciliation: boolean,
): Promise<void> {
	const { chain, solver } = tracked

	await recordDelegation(chain, solver, reading, blockNumber, at)

	for (const token of reading.tokens) {
		let vaults = 0n
		let vaultShares = 0n
		let drift = 0n
		for (const vault of token.vaults) {
			const position = await SolverVaultShares.get(vaultSharesId(chain, vault.vault, solver))
			const moved = !position || position.shares !== vault.shares || position.assets !== vault.assets
			await SolverVaultShares.create({
				id: vaultSharesId(chain, vault.vault, solver),
				chain,
				solver,
				vault: vault.vault,
				tokenAddress: token.token,
				shares: vault.shares,
				assets: vault.assets,
				blockNumber,
				lastLogIndex: AFTER_EVERY_LOG,
				observedAt: position ? (moved ? later(position.observedAt, at) : position.observedAt) : at,
			}).save()
			vaults += vault.assets
			vaultShares += vault.shares
			drift += vault.drift
		}

		const existing = await SolverInventory.get(inventoryId(chain, token.token, solver))
		const balance = token.wallet + vaults
		if (existing) drift += token.wallet - existing.wallet
		if (reconciliation && drift !== 0n) {
			logger.warn(
				`[solver-inventory] Reconciliation drift ${drift} for ${solver} in ${token.token} on ${chain} at block ${blockNumber}`,
			)
		}
		await SolverInventory.create({
			id: inventoryId(chain, token.token, solver),
			chain,
			solver,
			tokenAddress: token.token,
			balance,
			wallet: token.wallet,
			vaults,
			vaultShares,
			blockNumber,
			lastLogIndex: AFTER_EVERY_LOG,
			observedAt: existing
				? existing.balance !== balance
					? later(existing.observedAt, at)
					: existing.observedAt
				: at,
			refreshedAt: at,
			// A token added to the chain's config after the solver was tracked gets its genesis here.
			genesisBlock: existing?.genesisBlock ?? blockNumber,
			lastReadBlock: blockNumber,
			lastReconciledAt: reconciliation ? at : existing?.lastReconciledAt,
			lastReconciledDrift: reconciliation ? drift : existing?.lastReconciledDrift,
			discoveredBy: tracked.discoveredBy,
		}).save()
	}
}

/**
 * The genesis read for PENDING solvers, all of them in one batched read. A failed read leaves its
 * solver PENDING — nothing is written for it, and none of its events are applied meanwhile — so the
 * next block simply reads again, and that later read includes whatever moved in between. The other
 * solvers' readings are still applied.
 */
async function seedPendingSolvers(chain: string, blockNumber: bigint, at: Date): Promise<void> {
	const pending = await TrackedSolver.getByFields(
		[
			["chain", "=", chain],
			["status", "=", TrackedSolverStatus.PENDING],
		],
		{ limit: MAX_GENESIS_PER_BLOCK, orderBy: "id", orderDirection: "ASC" },
	)
	if (pending.length === 0) return
	let readings: (SolverReading | Error)[]
	try {
		readings = await readSolvers(
			chain,
			pending.map(({ solver }) => solver),
			false,
		)
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.warn(`[solver-inventory] Genesis reads failed on ${chain}, retrying next block: ${message}`)
		return
	}
	for (const [index, tracked] of pending.entries()) {
		const reading = readings[index]
		if (reading instanceof Error) {
			logger.warn(
				`[solver-inventory] Genesis read failed for ${tracked.solver} on ${chain}, retrying next block: ${reading.message}`,
			)
			continue
		}
		await applyReading(tracked, reading, blockNumber, at, false)
		tracked.status = TrackedSolverStatus.TRACKED
		tracked.genesisBlock = blockNumber
		tracked.revaluedAt = at
		tracked.reconciledAt = at
		await tracked.save()
		;(await trackedSolvers(chain)).add(tracked.solver)
	}
}

// ─── Events ─────────────────────────────────────────────────────────────────────────────────────

export interface TransferInput {
	chain: string
	from: string
	to: string
	blockNumber: bigint
	logIndex: number
	/** Resolved only when a tracked solver is involved: fetching it is an RPC. */
	timestamp: () => Promise<bigint>
}

/** Moves the wallet balance of whichever side of a supported-token Transfer is a tracked solver. */
export async function applyTokenTransfer(input: TransferInput & { token: string; value: bigint }): Promise<void> {
	const from = input.from.toLowerCase()
	const to = input.to.toLowerCase()
	if (from === to || input.value === 0n) return
	const tracked = await trackedSolvers(input.chain)
	const parties = [from, to].filter((address) => tracked.has(address))
	if (parties.length === 0) return

	const token = input.token.toLowerCase()
	let at: Date | undefined
	for (const solver of parties) {
		const row = await SolverInventory.get(inventoryId(input.chain, token, solver))
		if (!row || !isAfter(input.blockNumber, input.logIndex, row)) continue
		at ??= timestampToDate(await input.timestamp())
		let wallet = row.wallet + (solver === from ? -input.value : input.value)
		if (wallet < 0n) {
			logger.error(
				`[solver-inventory] ${solver}'s ${token} wallet on ${input.chain} went negative at block ${input.blockNumber}; clamping to zero until reconciliation`,
			)
			wallet = 0n
		}
		row.wallet = wallet
		row.balance = wallet + row.vaults
		row.blockNumber = input.blockNumber
		row.lastLogIndex = input.logIndex
		row.observedAt = later(row.observedAt, at)
		await row.save()
	}
}

/**
 * Moves the vault shares of whichever side of a vault share Transfer is a tracked solver. Mints and
 * burns count: a deposit is a mint to the owner, a withdrawal a burn from it.
 */
export async function applyVaultShareTransfer(input: TransferInput & { vault: string; shares: bigint }): Promise<void> {
	const from = input.from.toLowerCase()
	const to = input.to.toLowerCase()
	if (from === to || input.shares === 0n) return
	const tracked = await trackedSolvers(input.chain)
	const parties = [from, to].filter((address) => address !== ZERO_ADDRESS && tracked.has(address))
	if (parties.length === 0) return
	const vault = input.vault.toLowerCase()
	const token = tokenForVault(input.chain, vault)
	if (!token) return

	let at: Date | undefined
	for (const solver of parties) {
		const position = await SolverVaultShares.get(vaultSharesId(input.chain, vault, solver))
		if (!position || !isAfter(input.blockNumber, input.logIndex, position)) continue
		at ??= timestampToDate(await input.timestamp())
		let shares = position.shares + (solver === from ? -input.shares : input.shares)
		if (shares < 0n) {
			logger.error(
				`[solver-inventory] ${solver}'s shares in ${vault} on ${input.chain} went negative at block ${input.blockNumber}; clamping to zero until reconciliation`,
			)
			shares = 0n
		}
		position.shares = shares
		position.assets = unwrap((await valueShares(input.chain, [{ vault, shares }]))[0])
		position.blockNumber = input.blockNumber
		position.lastLogIndex = input.logIndex
		position.observedAt = later(position.observedAt, at)
		await position.save()
		await refoldVaults(input.chain, token, solver, at)
	}
}

/** Recomputes SolverInventory.vaults from the solver's positions in the token's vaults. */
async function refoldVaults(chain: string, token: string, solver: string, at: Date): Promise<void> {
	const inventory = await SolverInventory.get(inventoryId(chain, token, solver))
	if (!inventory) return
	let vaults = 0n
	let vaultShares = 0n
	for (const vault of supportedTokens(chain).find((entry) => entry.token === token)?.vaults ?? []) {
		const position = await SolverVaultShares.get(vaultSharesId(chain, vault, solver))
		vaults += position?.assets ?? 0n
		vaultShares += position?.shares ?? 0n
	}
	const balance = inventory.wallet + vaults
	if (balance !== inventory.balance) inventory.observedAt = later(inventory.observedAt, at)
	inventory.vaults = vaults
	inventory.vaultShares = vaultShares
	inventory.balance = balance
	await inventory.save()
}

// ─── Head and refresh ───────────────────────────────────────────────────────────────────────────

interface Revaluation {
	token: string
	positions: { position: SolverVaultShares; assets: bigint }[]
}

/** Values every solver's vault positions at this block, all of them in one batch. */
async function readRevaluations(chain: string, solvers: TrackedSolver[]): Promise<(Revaluation[] | Error)[]> {
	const amounts: ShareAmount[] = []
	const plans: { token: string; positions: { position: SolverVaultShares; slot: number }[] }[][] = []
	for (const { solver } of solvers) {
		const plan: (typeof plans)[number] = []
		for (const { token, vaults } of supportedTokens(chain)) {
			const positions: { position: SolverVaultShares; slot: number }[] = []
			for (const vault of vaults) {
				const position = await SolverVaultShares.get(vaultSharesId(chain, vault, solver))
				if (position) positions.push({ position, slot: amounts.push({ vault, shares: position.shares }) - 1 })
			}
			if (positions.length > 0) plan.push({ token, positions })
		}
		plans.push(plan)
	}
	const values = await valueShares(chain, amounts)
	return plans.map((plan) =>
		settleSolver(() =>
			plan.map(({ token, positions }) => ({
				token,
				positions: positions.map(({ position, slot }) => ({ position, assets: unwrap(values[slot]) })),
			})),
		),
	)
}

/** Writes a solver's revalued vault shares. Shares do not move, so the wallet is left alone. */
async function applyRevaluation(tracked: TrackedSolver, revalued: Revaluation[], at: Date): Promise<void> {
	const { chain, solver } = tracked
	for (const { token, positions } of revalued) {
		for (const { position, assets } of positions) {
			if (position.assets === assets) continue
			position.assets = assets
			position.observedAt = later(position.observedAt, at)
			await position.save()
		}
		await refoldVaults(chain, token, solver, at)
		const inventory = await SolverInventory.get(inventoryId(chain, token, solver))
		if (!inventory) continue
		inventory.refreshedAt = at
		await inventory.save()
	}
}

/**
 * One page of the chain's tracked solvers: each that is due is reconciled (daily) or has its vault
 * shares revalued (hourly). The page's reads are batched — the reconciliations' balances and
 * valuations, and the revaluations' valuations — so a page costs the same few eth_calls however
 * many of its solvers are due. Nothing is written for a solver until all of its reads have
 * succeeded.
 *
 * A read that fails for one solver skips only that solver, and the offset still advances: such a
 * failure is usually a revert that will keep failing, and holding the page for it would starve
 * every solver behind it. The solver stays due, so the next pass over its page reads it again. A
 * failed batch is the transient case — the RPC is down — and keeps the offset for the next advance.
 *
 * @returns the offset the next pass resumes from.
 */
async function refreshPage(chain: string, blockNumber: bigint, at: Date, offset: number): Promise<number> {
	const page = await TrackedSolver.getByFields(
		[
			["chain", "=", chain],
			["status", "=", TrackedSolverStatus.TRACKED],
		],
		{ limit: REFRESH_PAGE_SIZE, offset, orderBy: "id", orderDirection: "ASC" },
	)
	const reconciling: TrackedSolver[] = []
	const revaluing: TrackedSolver[] = []
	for (const tracked of page) {
		if (elapsedSecs(tracked.reconciledAt, at) >= RECONCILE_INTERVAL_SECS) reconciling.push(tracked)
		else if (elapsedSecs(tracked.revaluedAt, at) >= REVALUE_INTERVAL_SECS) revaluing.push(tracked)
	}

	let readings: (SolverReading | Error)[]
	let revaluations: (Revaluation[] | Error)[]
	try {
		;[readings, revaluations] = await Promise.all([
			readSolvers(
				chain,
				reconciling.map(({ solver }) => solver),
				true,
			),
			readRevaluations(chain, revaluing),
		])
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.warn(`[solver-inventory] Refresh reads failed on ${chain}, retrying next pass: ${message}`)
		return offset
	}

	const skip = (tracked: TrackedSolver, error: Error) =>
		logger.warn(
			`[solver-inventory] Refresh failed for ${tracked.solver} on ${chain}, retrying when its page comes round: ${error.message}`,
		)
	for (const [index, tracked] of reconciling.entries()) {
		const reading = readings[index]
		if (reading instanceof Error) {
			skip(tracked, reading)
			continue
		}
		await applyReading(tracked, reading, blockNumber, at, true)
		tracked.reconciledAt = at
		tracked.revaluedAt = at
		await tracked.save()
	}
	for (const [index, tracked] of revaluing.entries()) {
		const revaluation = revaluations[index]
		if (revaluation instanceof Error) {
			skip(tracked, revaluation)
			continue
		}
		await applyRevaluation(tracked, revaluation, at)
		tracked.revaluedAt = at
		await tracked.save()
	}
	return page.length < REFRESH_PAGE_SIZE ? 0 : offset + page.length
}

/**
 * Advances the chain's SolverInventoryHead, at most every HEAD_INTERVAL_SECS of block time, and
 * runs one refresh page with it. The head is what tells a consumer that an unchanged row is still
 * current: the rows themselves only move when an event does.
 */
async function advanceHead(chain: string, blockNumber: bigint, at: Date): Promise<void> {
	const head = await SolverInventoryHead.get(chain)
	if (head && elapsedSecs(head.observedAt, at) < HEAD_INTERVAL_SECS) return
	const next = head ?? SolverInventoryHead.create({ id: chain, blockNumber, observedAt: at, refreshOffset: 0 })
	next.refreshOffset = await refreshPage(chain, blockNumber, at, next.refreshOffset)
	next.blockNumber = blockNumber
	next.observedAt = at
	await next.save()
}

/** Everything the chain's EVM node does once per block for solver inventory. */
export async function indexSolverInventoryBlock(chain: string, blockNumber: bigint, timestamp: bigint): Promise<void> {
	if (supportedTokens(chain).length === 0) return
	const at = timestampToDate(timestamp)
	await consumeWatchlist(chain, blockNumber, at)
	await seedPendingSolvers(chain, blockNumber, at)
	await advanceHead(chain, blockNumber, at)
}
