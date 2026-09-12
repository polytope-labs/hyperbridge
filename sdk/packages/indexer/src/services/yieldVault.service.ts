import { ethers } from "ethers"

import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import {
	InventoryReadingTrigger,
	LiquidityProvider,
	VaultLedgerEvent,
	VaultLedgerEventType,
	VaultLpPosition,
	VaultPositionSnapshot,
	VaultSnapshot,
} from "@/configs/src/types"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { SOLVER_ACCOUNT_ADDRESSES } from "@/solver-account-addresses"
import { timestampToDate } from "@/utils/date.helpers"
import { publishProviderInventory } from "@/services/inventoryReading.service"
import { inventoryReadContext } from "@/utils/solverBalance"
import { isOrdinaryVaultTransfer, readVaultBlockMovements, type VaultCapitalMovement } from "@/utils/vaultAccounting"

const SECONDS_PER_DAY = 86400n

const DELEGATION_INDICATOR_PREFIX = "0xef0100"

// Page size for streaming an LP set out of the store during a snapshot. The store caps a single
// getByFields page, so positions are read in batches and snapshotted per page.
const LP_PAGE_SIZE = 100

/** A single supported vault on a chain, paired with the underlying token it wraps. */
export interface ConfiguredVault {
	vault: string
	underlyingToken: string
}

/** Inputs for recording one deposit, withdrawal or side of an ordinary share transfer. */
export interface VaultLedgerInput {
	chain: string
	/** The vault address the event was emitted by (event.address). */
	vault: string
	/** The share owner whose principal moved (Deposit.owner / Withdraw.owner). */
	lp: string
	/** Deposit.sender / Withdraw.sender, or Transfer.from for ordinary share transfers. */
	caller: string
	/** Withdraw.receiver or Transfer.to. Omitted for deposits. */
	receiver?: string
	assets: bigint
	shares: bigint
	eventType: VaultLedgerEventType
	blockNumber: bigint
	transactionHash: string
	logIndex: number
	/** Block timestamp in UNIX seconds. */
	timestamp: bigint
}

export interface VaultTransferInput
	extends Omit<VaultLedgerInput, "lp" | "caller" | "receiver" | "assets" | "eventType"> {
	from: string
	to: string
}

function isCapitalIn(type: VaultLedgerEventType): boolean {
	return type === VaultLedgerEventType.DEPOSIT || type === VaultLedgerEventType.TRANSFER_IN
}

function capitalEventId(input: Pick<VaultLedgerInput, "chain" | "transactionHash" | "logIndex">, lp?: string): string {
	return `${input.chain}-${input.transactionHash.toLowerCase()}-${input.logIndex}${lp ? `-${lp.toLowerCase()}` : ""}`
}

export class YieldVaultService {
	/**
	 * Resolve the underlying token a vault wraps from the generated config, case-insensitively.
	 * Returns undefined if the vault is not configured for the chain (e.g. a stale datasource).
	 */
	static underlyingTokenFor(chain: string, vault: string): string | undefined {
		const byToken = YIELD_VAULT_ADDRESSES[chain]
		if (!byToken) return undefined
		const target = vault.toLowerCase()
		for (const [token, vaults] of Object.entries(byToken)) {
			if (vaults.some((v) => v.toLowerCase() === target)) return token.toLowerCase()
		}
		return undefined
	}

	/** All supported vaults configured for a chain, lowercased. */
	static configuredVaults(chain: string): ConfiguredVault[] {
		const byToken = YIELD_VAULT_ADDRESSES[chain]
		if (!byToken) return []
		return Object.entries(byToken).flatMap(([token, vaults]) =>
			vaults.map((vault) => ({ vault: vault.toLowerCase(), underlyingToken: token.toLowerCase() })),
		)
	}

	/** Whether `lp` is delegated to one of our SolverAccounts at the handler's block. */
	static async isDelegatedSolver(chain: string, lp: string): Promise<boolean> {
		const solverAccounts = SOLVER_ACCOUNT_ADDRESSES[chain]
		if (!solverAccounts?.length) return false

		try {
			const code: string = await (api as any).getCode(lp)
			if (!code || !code.toLowerCase().startsWith(DELEGATION_INDICATOR_PREFIX)) return false
			const delegatedTo = ("0x" + code.slice(DELEGATION_INDICATOR_PREFIX.length)).toLowerCase()
			return solverAccounts.some((solverAccount) => solverAccount.toLowerCase() === delegatedTo)
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error)
			logger.warn(`[yield-vault] Delegation check failed for ${lp} on ${chain}: ${message}`)
			// A failed RPC is not evidence that this is an unrelated wallet. Retry the block instead
			// of permanently dropping a capital movement and overstating the next snapshot's yield.
			throw error
		}
	}

	private static async tracksLp(chain: string, vault: string, lp: string): Promise<boolean> {
		return (
			!!(await VaultLpPosition.get(`${chain}-${vault}-${lp}`)) ||
			!!(await LiquidityProvider.get(lp)) ||
			(await this.isDelegatedSolver(chain, lp))
		)
	}

	/** A share transfer is capital moving between owners, valued at this block's exchange rate. */
	static async recordTransfer(input: VaultTransferInput): Promise<void> {
		const from = input.from.toLowerCase()
		const to = input.to.toLowerCase()
		const vault = input.vault.toLowerCase()
		if (!isOrdinaryVaultTransfer(from, to, input.shares)) return
		if (!this.underlyingTokenFor(input.chain, vault)) return
		const owners: string[] = []
		for (const lp of [from, to]) {
			if (await VaultLedgerEvent.get(capitalEventId(input, lp))) continue
			if (await this.tracksLp(input.chain, vault, lp)) owners.push(lp)
		}
		if (!owners.length) return
		const contract = new ethers.Contract(vault, Erc4626Abi, api as any)
		const assets = BigInt((await contract.convertToAssets(input.shares.toString())).toString())
		// Sequential, with a distinct ledger key per owner: one log can affect two tracked LPs.
		for (const lp of owners) {
			await this.recordLedger({
				...input,
				vault,
				lp,
				caller: from,
				receiver: to,
				assets,
				eventType: lp === from ? VaultLedgerEventType.TRANSFER_OUT : VaultLedgerEventType.TRANSFER_IN,
			})
		}
	}

	/**
	 * Seed capital held before tracking began. balanceOf is end-of-block, so subtract ALL later
	 * capital movements in that block, including the triggering event. Subtracting only that
	 * event would count a later deposit/transfer twice (or lose a later withdrawal).
	 */
	private static async openingBalance(input: VaultLedgerInput): Promise<{ shares: bigint; assets: bigint }> {
		const movements = await readVaultBlockMovements(input.chain, input.vault, input.blockNumber)
		const delta = isCapitalIn(input.eventType) ? input.shares : -input.shares
		if (
			!movements.some(
				(m) =>
					m.lp === input.lp &&
					m.logIndex === input.logIndex &&
					m.transactionHash === input.transactionHash.toLowerCase() &&
					m.shares === delta,
			)
		) {
			throw new Error(
				`Vault opening balance is missing the triggering log for ${input.chain}:${input.vault}:${input.lp}`,
			)
		}
		const remainingShares = movements
			.filter((m) => m.lp === input.lp && m.logIndex >= input.logIndex)
			.reduce((sum, m) => sum + m.shares, 0n)
		const contract = new ethers.Contract(input.vault, Erc4626Abi, api as any)
		const shares = BigInt((await contract.balanceOf(input.lp)).toString()) - remainingShares
		if (shares < 0n) throw new Error(`Negative vault opening shares for ${input.chain}:${input.vault}:${input.lp}`)
		const assets = shares === 0n ? 0n : BigInt((await contract.convertToAssets(shares.toString())).toString())
		return { shares, assets }
	}

	/**
	 * Persist one capital movement and fold it into the LP's running position. Opening capital,
	 * deposits/withdrawals and transfers form the baseline the daily snapshot measures yield against.
	 */
	static async recordLedger(input: VaultLedgerInput): Promise<void> {
		const vault = input.vault.toLowerCase()
		const lp = input.lp.toLowerCase()
		const underlyingToken = this.underlyingTokenFor(input.chain, vault)
		if (!underlyingToken) {
			logger.warn(`[yield-vault] Unconfigured vault ${vault} on ${input.chain}, skipping ledger event`)
			return
		}

		// Only track our solvers. LiquidityProvider is written by the phantom handler on another node
		// and may lag a solver's first deposit, so fall back to the on-chain delegation check.
		const positionId = `${input.chain}-${vault}-${lp}`
		const existingPosition = await VaultLpPosition.get(positionId)
		if (
			!existingPosition &&
			!(await LiquidityProvider.get(lp)) &&
			!(await this.isDelegatedSolver(input.chain, lp))
		) {
			return
		}

		// Idempotency guard: the position is folded with += / -=, so applying the same event twice would
		// corrupt principal. Skip if this exact log was already recorded. (A reorg rolls back both the
		// ledger row and the position together under historical indexing, so legitimate reprocessing
		// still re-applies cleanly — this only blocks true duplicate delivery.)
		const isTransfer =
			input.eventType === VaultLedgerEventType.TRANSFER_IN ||
			input.eventType === VaultLedgerEventType.TRANSFER_OUT
		const ledgerId = capitalEventId(input, isTransfer ? lp : undefined)
		if (await VaultLedgerEvent.get(ledgerId)) {
			logger.debug(`[yield-vault] Ledger event ${ledgerId} already recorded, skipping`)
			return
		}

		const eventTime = timestampToDate(input.timestamp)
		// Complete every required RPC before persisting the dedup row.
		const opening = existingPosition ? undefined : await this.openingBalance({ ...input, lp, vault })

		await VaultLedgerEvent.create({
			id: ledgerId,
			chain: input.chain,
			vault,
			underlyingToken,
			lp,
			caller: input.caller.toLowerCase(),
			receiver: input.receiver?.toLowerCase(),
			eventType: input.eventType,
			assets: input.assets,
			shares: input.shares,
			blockNumber: input.blockNumber,
			transactionHash: input.transactionHash.toLowerCase(),
			timestamp: eventTime,
		}).save()

		let position = existingPosition
		if (!position) {
			position = VaultLpPosition.create({
				id: positionId,
				chain: input.chain,
				vault,
				underlyingToken,
				lp,
				shares: opening!.shares,
				openingShares: opening!.shares,
				openingPrincipal: opening!.assets,
				openingBlock: input.blockNumber,
				totalAssetsTransferredIn: 0n,
				totalAssetsTransferredOut: 0n,
				totalAssetsDeposited: 0n,
				totalAssetsWithdrawn: 0n,
				depositCount: 0,
				withdrawCount: 0,
				createdAt: eventTime,
				lastUpdatedAt: eventTime,
			})
		}

		if (input.eventType === VaultLedgerEventType.DEPOSIT) {
			position.totalAssetsDeposited += input.assets
			position.shares += input.shares
			position.depositCount += 1
		} else if (input.eventType === VaultLedgerEventType.WITHDRAW) {
			position.totalAssetsWithdrawn += input.assets
			position.shares -= input.shares
			position.withdrawCount += 1
		} else if (input.eventType === VaultLedgerEventType.TRANSFER_IN) {
			position.totalAssetsTransferredIn = (position.totalAssetsTransferredIn ?? 0n) + input.assets
			position.shares += input.shares
		} else {
			position.totalAssetsTransferredOut = (position.totalAssetsTransferredOut ?? 0n) + input.assets
			position.shares -= input.shares
		}
		position.lastUpdatedAt = eventTime

		await position.save()

		// The LP's inventory in this token just moved: its own principal only shifted between the
		// raw and vault halves of one total, which the re-read confirms rather than changes, but the
		// total does move when the counterparty is someone else (a treasury funding the solver, or
		// inventory leaving it) and no order event reports that at all. Best-effort — this reads
		// external RPCs, and the ledger row above must not be lost to a publication failure.
		try {
			await publishProviderInventory({
				provider: lp,
				tokens: [underlyingToken],
				...inventoryReadContext(input.chain, input.blockNumber, input.timestamp, InventoryReadingTrigger.VAULT),
			})
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error)
			logger.error(`[yield-vault] Inventory publication failed for ${lp} on ${input.chain}: ${message}`)
		}
	}

	/**
	 * Take the daily snapshot for every configured vault on a chain: a vault-level snapshot
	 * (totalAssets / totalShares / assetsPerShare) plus one per-LP snapshot pricing each LP's live
	 * share balance into assets via `convertToAssets`. Idempotent per UTC day — a vault whose
	 * day-bucket snapshot already exists is skipped before any RPC, so re-attempts are cheap.
	 */
	static async snapshotChain(chain: string, blockNumber: bigint, timestamp: bigint): Promise<void> {
		const dayStart = (timestamp / SECONDS_PER_DAY) * SECONDS_PER_DAY
		const snapshotTime = timestampToDate(timestamp)

		for (const { vault, underlyingToken } of this.configuredVaults(chain)) {
			const vaultSnapshotId = `${chain}-${vault}-${dayStart}`
			if (await VaultSnapshot.get(vaultSnapshotId)) continue

			const contract = new ethers.Contract(vault, Erc4626Abi, api as any)
			// Ethereum block handlers run BEFORE this block's log handlers. Even a zero-net-share
			// round trip can change principal, so do not snapshot an LP touched by this block.
			let movements: VaultCapitalMovement[]

			let totalAssets: bigint
			let totalShares: bigint
			let assetsPerShare: bigint
			try {
				movements = await readVaultBlockMovements(chain, vault, blockNumber)
				const [assetsRaw, supplyRaw, decimalsRaw] = await Promise.all([
					contract.totalAssets(),
					contract.totalSupply(),
					contract.decimals(),
				])
				const oneShare = ethers.BigNumber.from(10).pow(decimalsRaw)
				const perShareRaw = await contract.convertToAssets(oneShare)
				totalAssets = BigInt(assetsRaw.toString())
				totalShares = BigInt(supplyRaw.toString())
				assetsPerShare = BigInt(perShareRaw.toString())
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				logger.error(`[yield-vault] Vault read failed for ${vault} on ${chain}: ${message}`)
				continue
			}

			// Snapshot the LPs first, then write the vault-level row last. The vault snapshot is the
			// per-day completion gate (checked at the top), so persisting it only after the LP loop
			// finishes means a mid-loop failure leaves the gate open and the next run retries — the
			// per-LP dedup skips LPs already done, so the remainder is filled in rather than lost.
			if (
				!(await this.snapshotLpPositions(
					chain,
					vault,
					underlyingToken,
					contract,
					dayStart,
					blockNumber,
					snapshotTime,
					movements,
				))
			)
				continue

			await VaultSnapshot.create({
				id: vaultSnapshotId,
				chain,
				vault,
				underlyingToken,
				dayStartTimestamp: dayStart,
				totalAssets,
				totalShares,
				assetsPerShare,
				blockNumber,
				snapshotTime,
			}).save()
		}
	}

	private static async snapshotLpPositions(
		chain: string,
		vault: string,
		underlyingToken: string,
		contract: ethers.Contract,
		dayStart: bigint,
		blockNumber: bigint,
		snapshotTime: Date,
		movements: VaultCapitalMovement[],
	): Promise<boolean> {
		let offset = 0
		let complete = true
		// Stream the vault's LPs page by page so a vault with many positions doesn't load them all at once.
		for (;;) {
			const positions = await VaultLpPosition.getByFields(
				[
					["chain", "=", chain],
					["vault", "=", vault],
				],
				// Order by the primary key so offset paging is stable and never skips an LP across pages.
				{ limit: LP_PAGE_SIZE, offset, orderBy: "id", orderDirection: "ASC" },
			)
			if (positions.length === 0) break

			const results = await Promise.all(
				positions.map((position) =>
					this.snapshotLpPosition(
						position,
						chain,
						vault,
						underlyingToken,
						contract,
						dayStart,
						blockNumber,
						snapshotTime,
						movements,
					),
				),
			)
			if (results.includes("retry")) complete = false

			if (positions.length < LP_PAGE_SIZE) break
			offset += LP_PAGE_SIZE
		}
		return complete
	}

	/** Price one LP's live share balance into assets and persist its daily snapshot. */
	private static async snapshotLpPosition(
		position: VaultLpPosition,
		chain: string,
		vault: string,
		underlyingToken: string,
		contract: ethers.Contract,
		dayStart: bigint,
		blockNumber: bigint,
		snapshotTime: Date,
		movements: VaultCapitalMovement[],
	): Promise<"complete" | "retry" | "unreconciled"> {
		const snapshotId = `${chain}-${vault}-${position.lp}-${dayStart}`
		if (await VaultPositionSnapshot.get(snapshotId)) return "complete"
		if (movements.some((m) => m.lp === position.lp)) return "retry"

		let shares: bigint
		let assetValue: bigint
		try {
			const sharesRaw = await contract.balanceOf(position.lp)
			const assetsRaw = await contract.convertToAssets(sharesRaw)
			shares = BigInt(sharesRaw.toString())
			assetValue = BigInt(assetsRaw.toString())
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error)
			logger.error(`[yield-vault] LP read failed for ${position.lp} on ${vault}: ${message}`)
			return "retry"
		}

		if (shares !== position.shares) {
			logger.error(
				`[yield-vault] Withholding yield for ${chain}:${vault}:${position.lp}: ` +
					`accounted shares=${position.shares}, onchain shares=${shares}; principal requires reconciliation`,
			)
			// This needs a historical repair, not another RPC in 50 blocks. Do not prevent the
			// independent vault aggregate from being published or repeatedly scan every LP today.
			return "unreconciled"
		}

		const netPrincipal =
			(position.openingPrincipal ?? 0n) +
			position.totalAssetsDeposited -
			position.totalAssetsWithdrawn +
			(position.totalAssetsTransferredIn ?? 0n) -
			(position.totalAssetsTransferredOut ?? 0n)
		const yieldEarned = assetValue - netPrincipal
		if (yieldEarned < 0n) {
			// Expected transiently (a fresh deposit before yield accrues, or a vesting/loss vault), but
			// a persistent negative can also signal ledger/contract drift, so surface it.
			logger.warn(
				`[yield-vault] Negative yield for ${chain}:${vault}:${position.lp} ` +
					`(assetValue=${assetValue}, netPrincipal=${netPrincipal})`,
			)
		}

		await VaultPositionSnapshot.create({
			id: snapshotId,
			chain,
			vault,
			underlyingToken,
			lp: position.lp,
			dayStartTimestamp: dayStart,
			shares,
			assetValue,
			netPrincipal,
			yieldEarned,
			blockNumber,
			snapshotTime,
		}).save()
		return "complete"
	}
}
