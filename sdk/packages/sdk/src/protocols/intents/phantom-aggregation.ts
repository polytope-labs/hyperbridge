// Phantom-order price/liquidity aggregation. Lives in the SDK so the indexer (which persists the
// result as entities) and the simplex E2E test share one implementation. Bid decoding (extractFill)
// and signature recovery (recoverSigner) are injectable because the indexer must do both without
// viem — viem's @noble/hashes keccak throws "Uint8Array expected" in the SubQuery VM2 sandbox — so
// it passes VM2-safe implementations; the viem-based defaults are fine for Node consumers (tests,
// simplex).
import { decodeFunctionData, encodeAbiParameters, keccak256, recoverAddress } from "viem"
import { hexToU8a, isHex, stringToU8a, u8aToHex, u8aToString } from "@polkadot/util"
import { decodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { decodeUserOpScale } from "@/chains/intentsCoprocessor"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { PackedUserOperation } from "@/types"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"

export type HexString = `0x${string}`

/**
 * ERC-4337 v0.8 EntryPoint, the contract whose userOpHash a bid's solver signature is taken over.
 * Canonical across every EVM chain we support, which is why it is a constant here rather than
 * something callers pass in (chain.ts carries the same address per chain as `EntryPointV08`).
 */
export const ENTRY_POINT_V08_ADDRESS = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as const

/** Minimal fetch shape used by the JSON-RPC POSTs below. */
export type FetchLike = (url: string, init: any) => Promise<{ json(): Promise<any> }>

// The aggregation talks to RPCs over HTTP. In browsers/Node/tests the global `fetch` is used, but
// the SubQuery VM2 sandbox the indexer runs in does NOT expose a global `fetch` (and node-fetch
// crashes there), so the indexer injects a sandbox-safe implementation via setAggregationFetch().
let injectedFetch: FetchLike | undefined
export function setAggregationFetch(fetchImpl: FetchLike): void {
	injectedFetch = fetchImpl
}
function rpcFetch(): FetchLike {
	const f = injectedFetch ?? (globalThis as { fetch?: FetchLike }).fetch
	if (typeof f !== "function") {
		throw new Error("No fetch available; call setAggregationFetch() before using the aggregation helpers")
	}
	return f
}

// POSTs a JSON-RPC payload and returns the parsed response, retrying with a short backoff. The node
// intermittently returns an empty body under concurrent load (a 200 with no payload), which makes
// response.json() throw; without a retry a single blip would silently drop a bid's quote or a whole
// window (fetchBids throws). Throws if every attempt fails.
async function rpcCall(url: string, payload: object): Promise<any> {
	let lastErr: unknown
	for (let attempt = 0; attempt < 4; attempt++) {
		if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 150 * attempt))
		let timer: ReturnType<typeof setTimeout> | undefined
		try {
			// Bound each attempt: the injected fetch (Node http) has no socket timeout, so a stalled
			// connection would otherwise hang forever and block the whole handler. Race it against a
			// deadline; on timeout we reject, retry, and ultimately throw so callers degrade instead.
			const timeout = new Promise<never>((_, reject) => {
				timer = setTimeout(() => reject(new Error(`rpc timeout: ${url}`)), 12_000)
			})
			const response = await Promise.race([
				rpcFetch()(url, {
					method: "POST",
					headers: { accept: "application/json", "content-type": "application/json" },
					body: JSON.stringify(payload),
				}),
				timeout,
			])
			return await response.json()
		} catch (err) {
			lastErr = err
		} finally {
			if (timer) clearTimeout(timer)
		}
	}
	throw lastErr
}

export const FILL_ORDER_ABI = IntentGatewayV2.ABI

// ─── accepted-source-chains declaration ─────────────────────────────────────────────────────────
//
// A same-chain phantom bid proves a solver operates on a chain, not which chains it will accept
// payment FROM when filling a cross-chain order. Bids declare that set inside paymasterAndData,
// which the userOpHash covers, so the declaration is authenticated by the solver's existing bid
// signature (the `signature` field is excluded from the hash and therefore unusable). The
// overload applies to phantom bids only: a real fill's paymasterAndData keeps its functional
// EntryPoint semantics, and nothing on the real-fill path ever parses this format.
//
// Layout: version(1) ‖ count(1) ‖ count × (length(1) ‖ utf8 state machine id). A declaration
// with no entries is a deliberate "accepts no source chains" and is distinct from an absent
// declaration ("0x"), which means the legacy default: all CCTP/USDT0-covered chains.

const DECLARATION_VERSION = 0x01

/** Upper bound on declared chains; one byte of count, and far beyond any real deployment. */
const MAX_DECLARED_CHAINS = 255

/**
 * Encodes the accepted source chains (state machine ids, e.g. "EVM-8453") into the
 * paymasterAndData declaration blob.
 */
export function encodeAcceptedSourceChains(chains: string[]): HexString {
	if (chains.length > MAX_DECLARED_CHAINS) {
		throw new Error(`Cannot declare more than ${MAX_DECLARED_CHAINS} source chains`)
	}

	const bytes: number[] = [DECLARATION_VERSION, chains.length]
	for (const chain of chains) {
		const encoded = stringToU8a(chain)
		if (encoded.length === 0 || encoded.length > 255) {
			throw new Error(`Invalid state machine id in source chain declaration: ${chain}`)
		}
		bytes.push(encoded.length, ...encoded)
	}
	return u8aToHex(new Uint8Array(bytes)) as HexString
}

/**
 * Decodes a phantom bid's paymasterAndData into its declared source chains. Returns null for an
 * absent, unversioned or malformed blob — the legacy default — and an empty array only for an
 * explicit zero-entry declaration. Callers must preserve that distinction.
 */
export function decodeAcceptedSourceChains(paymasterAndData: string | undefined | null): string[] | null {
	if (!paymasterAndData || !isHex(paymasterAndData)) return null
	const bytes = hexToU8a(paymasterAndData)
	if (bytes.length < 2 || bytes[0] !== DECLARATION_VERSION) return null

	const count = bytes[1]
	const chains: string[] = []
	let offset = 2
	for (let entry = 0; entry < count; entry++) {
		if (offset >= bytes.length) return null
		const length = bytes[offset]
		offset += 1
		if (length === 0 || offset + length > bytes.length) return null
		chains.push(u8aToString(bytes.subarray(offset, offset + length)))
		offset += length
	}
	// Trailing bytes mean this is not a declaration but something that happens to share the
	// version byte, so treat the whole blob as unparseable rather than half-reading it.
	if (offset !== bytes.length) return null

	return chains
}

/** ERC-4626 vaults per chain, keyed by chain id then lowercase underlying token address. */
export type YieldVaultMap = Record<string, Record<string, string[]>>

/** One leg of a fill: the token the solver pays out and the amount it quoted for that leg. */
export interface FillLeg {
	outputToken: HexString
	solverAmount: bigint
}

export interface FillData {
	order: Record<string, unknown>
	options: Record<string, unknown>
	/** Positional, matching the order's asset lists. A zero amount means the solver did not quote that leg. */
	legs: FillLeg[]
}

/**
 * Zips an order's output assets with the bid's quoted amounts into positional legs. A missing or
 * null amount is the "solver declined this leg" sentinel and becomes a zero quote. Shared by the
 * viem extractor below and the indexer's VM2-safe one, so the declined-leg convention has a
 * single home while only the ABI decode differs per environment.
 */
export function zipFillLegs(assets: { token: HexString }[], outputs: { amount: unknown }[]): FillLeg[] {
	return assets.map((asset, index) => {
		const rawAmount = outputs[index]?.amount
		return {
			outputToken: asset.token,
			solverAmount: rawAmount === undefined || rawAmount === null ? 0n : BigInt(rawAmount.toString()),
		}
	})
}

export interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

/** One solver's measured liquidity for a configured token on one chain at this snapshot. */
export interface LpBalance {
	solver: string
	/** State machine id of the chain the balance was measured on (e.g. EVM-8453). */
	chain: string
	tokenAddress: HexString
	balance: bigint
}

/** One verified solver behind a leg's quote, holding inventory to deliver it. */
export interface PhantomLegBidder {
	solver: HexString
	/**
	 * The solver's output-token inventory on the destination chain — its weight in the median.
	 * Always greater than zero: a solver quoting a leg it holds none of is dropped, not recorded
	 * at zero, since it can deliver nothing at any price.
	 */
	weight: bigint
	/**
	 * Source chains the solver's signed paymasterAndData declaration accepts payment from. Null
	 * when the bid carries no declaration (legacy default: all CCTP/USDT0-covered chains); an
	 * empty array is an explicit accepts-nothing declaration.
	 */
	acceptedSources: string[] | null
}

/** The aggregated price for one leg of a phantom order. */
export interface PhantomLegAggregation {
	/** Position of the leg in the order's asset lists. */
	legIndex: number
	outputToken: HexString
	lowestPrice: bigint
	highestPrice: bigint
	medianPrice: bigint
	/** Backed quotes behind the price. Quotes from solvers holding no inventory are not counted. */
	bidCount: number
	/** The verified, inventory-backed solvers quoting this leg; bidCount === bidders.length. */
	bidders: PhantomLegBidder[]
}

/** The aggregated result for a single phantom order's bid window. */
export interface PhantomAggregation {
	/**
	 * One entry per leg that at least one solver quoted AND at least one of those quotes is backed
	 * by output-token inventory on the destination chain. Legs nobody quoted are absent, and so are
	 * legs every bidder quoted on zero inventory — neither is a price anyone could trade against.
	 */
	legs: PhantomLegAggregation[]
	lpBalances: LpBalance[]
}

export interface AggregationLogger {
	warn: (payload: unknown, message: string) => void
}

// Liquidity-weighted median of solver quotes. Each quote's influence is proportional to `weight` —
// the solver's total balance for the output token across native + vault venues — so a solver that
// can actually deliver size moves the price more than one quoting on thin liquidity. Returns the
// lower weighted median: the smallest price whose cumulative weight reaches half of the total.
// Zero-weight quotes contribute nothing.
//
// Callers must not hand this an entry set whose weights are all zero: with nothing to weight by it
// can only pick a quote by position, and for an even-sized set that position is the upper of the
// two middles — so "the median" becomes "whoever quoted higher", settable by a solver holding no
// inventory at all. aggregatePhantomBids drops such legs instead of pricing them. The fallback
// below stays only so an unguarded caller gets a number rather than a crash; treat reaching it as
// a caller bug.
export function weightedMedian(entries: { price: bigint; weight: bigint }[]): bigint {
	const sorted = [...entries].sort((a, b) => (a.price < b.price ? -1 : a.price > b.price ? 1 : 0))
	const totalWeight = sorted.reduce((acc, e) => (e.weight > 0n ? acc + e.weight : acc), 0n)

	if (totalWeight === 0n) {
		return sorted[Math.floor(sorted.length / 2)].price
	}

	let cumulative = 0n
	for (const entry of sorted) {
		if (entry.weight <= 0n) continue
		cumulative += entry.weight
		if (cumulative * 2n >= totalWeight) return entry.price
	}
	return sorted[sorted.length - 1].price
}

// Pulls the inner fillOrder call out of the bid's ERC-7821 execute batch and decodes the order, the
// offered output token, and the solver's quoted amount. Returns null when no matching call targets
// the gateway or the calldata cannot be decoded.
export function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			const decoded = decodeFunctionData({ abi: FILL_ORDER_ABI, data: call.data as HexString })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const order = decoded.args[0] as Record<string, unknown>
			const options = decoded.args[1] as Record<string, unknown>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const assets = (order as any)?.output?.assets as { token: HexString }[] | undefined
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputs = (options as any)?.outputs as { amount: bigint }[] | undefined
			if (!assets?.length || !outputs?.length) continue
			return { order, options, legs: zipFillLegs(assets, outputs) }
		} catch {
			continue
		}
	}
	return null
}

/** Derives the 192-bit bid nonce key binding a bid to an (order, sessionKey) pair. */
export type BidNonceKeyFn = (commitment: HexString, sessionKey: HexString) => bigint

/** Recomputes an order's commitment from the contract-shaped order decoded out of a bid's calldata. */
export type OrderCommitmentFn = (order: Record<string, unknown>) => HexString | null

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const FILL_ORDER_INPUT = (FILL_ORDER_ABI as readonly any[]).find(
	(item) => item?.type === "function" && item?.name === "fillOrder",
)?.inputs?.[0]

/**
 * Default (viem) {@link OrderCommitmentFn}. The order comes straight out of `fillOrder`'s ABI
 * decode, so it is already contract-shaped and re-encoding it reproduces `keccak256(abi.encode(order))`
 * — the same commitment IntentGatewayV2 computes on-chain. Returns null when it cannot be computed,
 * which callers treat as unverifiable (fail closed).
 */
export function orderCommitmentFromDecoded(order: Record<string, unknown>): HexString | null {
	try {
		if (!FILL_ORDER_INPUT) return null
		return keccak256(encodeAbiParameters([FILL_ORDER_INPUT], [order as never]))
	} catch {
		return null
	}
}

// A bid's userOp.signature is `commitment (32) ‖ solverSignature (65)`. SolverAccount.validateUserOp
// expects 162 bytes on-chain, but the trailing 65-byte session-key signature is only appended at fill
// time, so a bid is stored and read back in this 97-byte form.
const BID_COMMITMENT_BYTES = 32
const SOLVER_SIGNATURE_BYTES = 65

export interface BidSignature {
	/** The order commitment the solver signature is bound to. */
	commitment: HexString
	/** The solver's 65-byte ECDSA signature over the userOpHash. */
	solverSignature: HexString
}

/** Splits a bid userOp's signature into its commitment and solver signature; null if malformed. */
export function splitBidSignature(signature: HexString): BidSignature | null {
	const raw = signature.replace(/^0x/, "")
	const commitmentChars = BID_COMMITMENT_BYTES * 2
	const end = commitmentChars + SOLVER_SIGNATURE_BYTES * 2
	if (raw.length < end) return null

	return {
		commitment: `0x${raw.slice(0, commitmentChars)}` as HexString,
		solverSignature: `0x${raw.slice(commitmentChars, end)}` as HexString,
	}
}

/** Recovers the address that produced a bid's solver signature, or null if it cannot be recovered. */
export type RecoverBidSigner = (
	userOp: PackedUserOperation,
	entryPoint: HexString,
	chainId: bigint,
	solverSignature: HexString,
) => Promise<HexString | null>

/**
 * Default {@link RecoverBidSigner}: recovers over the EntryPoint v0.8 userOpHash, the digest
 * SolverAccount itself validates against. viem-based, so Node consumers get it for free while the
 * indexer injects an ethers equivalent (see the note at the top of this file).
 */
export const recoverBidSignerViem: RecoverBidSigner = async (userOp, entryPoint, chainId, solverSignature) => {
	try {
		const userOpHash = CryptoUtils.computeUserOpHash(userOp, entryPoint, chainId)
		return (await recoverAddress({ hash: userOpHash, signature: solverSignature })) as HexString
	} catch {
		return null
	}
}

// An EOA that has delegated with EIP-7702 has code `0xef0100 ‖ delegate`.
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/** Whether `account` is an EOA EIP-7702-delegated to `solverAccount` on the given chain. */
async function isDelegatedToSolverAccount(evmRpcUrl: string, account: string, solverAccount: string): Promise<boolean> {
	const response = await rpcCall(evmRpcUrl, {
		id: 1,
		jsonrpc: "2.0",
		method: "eth_getCode",
		params: [account, "latest"],
	})
	const code = typeof response.result === "string" ? response.result.toLowerCase() : ""
	if (!code.startsWith(DELEGATION_INDICATOR_PREFIX)) return false

	return `0x${code.slice(DELEGATION_INDICATOR_PREFIX.length)}` === solverAccount.toLowerCase()
}

/** Chain id out of an EVM state machine id ("EVM-8453" -> 8453n); null for any other format. */
function evmChainId(chain: string): bigint | null {
	const [prefix, id] = chain.split("-")
	if (prefix !== "EVM" || !id || !/^\d+$/.test(id)) return null
	return BigInt(id)
}

/**
 * Whether a bid genuinely came from one of our solvers, and so may influence the snapshot.
 *
 * Anyone can submit a bid to the coprocessor, and every accepted quote moves the weighted median the
 * rest of the protocol prices intents against, so a bid is only counted if it clears both of the
 * checks SolverAccount would apply on-chain: the userOp carries a solver signature over this order's
 * userOpHash that recovers to the sender, and the sender is EIP-7702-delegated to the chain's
 * SolverAccount. Fails closed — a bid that cannot be read or verified is not counted.
 */
async function isVerifiedSolverBid(params: {
	userOp: PackedUserOperation
	commitment: string
	sessionKey: HexString
	chainId: bigint
	solverAccount: string
	evmRpcUrl: string
	recoverSigner: RecoverBidSigner
	bidNonceKey: BidNonceKeyFn
	logger?: AggregationLogger
}): Promise<boolean> {
	const { userOp, commitment, sessionKey, chainId, solverAccount, evmRpcUrl, recoverSigner, bidNonceKey, logger } =
		params
	const solver = userOp.sender

	const parsed = splitBidSignature(userOp.signature)
	if (!parsed) {
		logger?.warn({ solver, commitment }, "Rejecting phantom bid: malformed userOp signature")
		return false
	}

	// Cheap early-out ONLY. The prefix sits inside userOp.signature, which userOpHash excludes, so it
	// is attacker-mutable and must never be what binds a bid to an order — the nonce key below is.
	if (parsed.commitment.toLowerCase() !== commitment.toLowerCase()) {
		logger?.warn(
			{ solver, commitment, signedFor: parsed.commitment },
			"Rejecting phantom bid: signed for another order",
		)
		return false
	}

	// The authoritative binding, mirroring SolverAccount.validateUserOp on-chain. The nonce IS
	// covered by userOpHash, so a solver signature stays valid only for the (order, sessionKey) pair
	// its nonce key was derived from. `sessionKey` is read from the bid's own calldata, which is also
	// covered by userOpHash — so every operand here is signed, leaving nothing for a replay to swap.
	if (BigInt(userOp.nonce) >> 64n !== bidNonceKey(commitment as HexString, sessionKey)) {
		logger?.warn({ solver, commitment }, "Rejecting phantom bid: nonce does not bind order and session key")
		return false
	}

	// SolverAccount._rawSignatureValidation recovers over the bare userOpHash and requires the signer
	// to be the account itself, which under EIP-7702 is the sender EOA.
	const signer = await recoverSigner(userOp, ENTRY_POINT_V08_ADDRESS, chainId, parsed.solverSignature)
	if (!signer || signer.toLowerCase() !== solver.toLowerCase()) {
		logger?.warn({ solver, commitment, signer }, "Rejecting phantom bid: signature does not recover to the sender")
		return false
	}

	if (!(await isDelegatedToSolverAccount(evmRpcUrl, solver, solverAccount))) {
		logger?.warn({ solver, commitment, solverAccount }, "Rejecting phantom bid: sender is not a delegated solver")
		return false
	}

	return true
}

export async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
	const data = await rpcCall(nodeUrl, {
		id: 1,
		jsonrpc: "2.0",
		method: "intents_getBidsForOrder",
		params: [commitment],
	})
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

async function ethCallUint(evmRpcUrl: string, to: string, data: string): Promise<bigint> {
	try {
		const result = await rpcCall(evmRpcUrl, {
			id: 1,
			jsonrpc: "2.0",
			method: "eth_call",
			params: [{ to, data }, "latest"],
		})
		if (result.error || !result.result || result.result === "0x") return 0n
		return BigInt(result.result)
	} catch {
		return 0n
	}
}

// Sums the solver's redeemable balance of a single token on its destination chain: the raw ERC-20
// balance plus any ERC-4626 vault positions wrapping it.
async function getTotalSolverBalance(
	evmRpcUrl: string,
	chain: string,
	token: string,
	solver: string,
	yieldVaults: YieldVaultMap,
): Promise<bigint> {
	const padded = solver.replace("0x", "").padStart(64, "0")
	const raw = await ethCallUint(evmRpcUrl, token, `0x70a08231${padded}`) // balanceOf(address)
	const vaults = yieldVaults[chain]?.[token.toLowerCase()] ?? []
	const vaultBalances = await Promise.all(
		vaults.map((v) => ethCallUint(evmRpcUrl, v, `0xce96cb77${padded}`)), // maxWithdraw(address)
	)
	return vaultBalances.reduce((acc, b) => acc + b, raw)
}

/** Promise-caching balance reader produced by [`memoizedSolverBalance`]. */
export type SolverBalanceReader = (
	evmRpcUrl: string,
	chain: string,
	token: string,
	solver: string,
) => Promise<bigint>

// One aggregation run reads the same (chain, token, solver) balance from several places — once
// per leg the solver quoted in that token, and again in the liquidity sweep — and a bundled order
// can carry up to 128 legs. Memoizing for the life of the run collapses that to one RPC round
// trip per distinct triple, at the cost of ignoring balance changes within the run (the snapshot
// is a point-in-time read either way). Exported so a caller aggregating several orders whose bid
// windows close on the same block (one bundled order per configured chain) can share one memo
// across the runs instead of re-reading identical balances per order.
export function memoizedSolverBalance(yieldVaults: YieldVaultMap): SolverBalanceReader {
	const cache = new Map<string, Promise<bigint>>()
	return (evmRpcUrl: string, chain: string, token: string, solver: string): Promise<bigint> => {
		const key = `${chain}|${token.toLowerCase()}|${solver.toLowerCase()}`
		let pending = cache.get(key)
		if (!pending) {
			pending = getTotalSolverBalance(evmRpcUrl, chain, token, solver, yieldVaults)
			cache.set(key, pending)
		}
		return pending
	}
}

// Sweeps a solver's liquidity for every configured yield-vault token on every supported chain: for
// each chain that has both an RPC (in evmRpcUrls) and configured tokens (in yieldVaults), the
// solver's balance (raw ERC-20 + ERC-4626 vault positions) for each token. Captures the LP's whole
// liquidity picture, not just the token of the bid being priced. Zero balances are skipped so the
// snapshot only records tokens the solver actually holds.
async function sweepSolverLiquidity(
	evmRpcUrls: Record<string, string>,
	yieldVaults: YieldVaultMap,
	solver: string,
	getBalance: ReturnType<typeof memoizedSolverBalance>,
): Promise<LpBalance[]> {
	const balances: LpBalance[] = []
	for (const [chain, tokens] of Object.entries(yieldVaults)) {
		const url = evmRpcUrls[chain]
		if (!url) continue
		for (const token of Object.keys(tokens)) {
			const balance = await getBalance(url, chain, token, solver)
			if (balance === 0n) continue
			balances.push({ solver, chain, tokenAddress: token as HexString, balance })
		}
	}
	return balances
}

// Strips a bytes32 token field to a 20-byte lowercase address (or normalises an address as-is).
function toAddress(token: string): HexString {
	const hex = token.toLowerCase().replace(/^0x/, "")
	const addr = hex.length > 40 ? hex.slice(-40) : hex.padStart(40, "0")
	return `0x${addr}` as HexString
}

/**
 * Aggregates every bid for a phantom order into a single price/liquidity snapshot.
 *
 * Fetches the live bids via `intents_getBidsForOrder` and reads each filler's quoted output amount.
 * Only bids that {@link isVerifiedSolverBid} accepts are counted — a bid from anyone who is not one
 * of our delegated solvers, or whose signature was not produced for this order, is dropped rather
 * than allowed to move the price. The liquidity-weighted median then weights every surviving quote by
 * the solver's balance of the output token on the destination chain, so a solver that can't actually
 * deliver size carries little or no weight — which is why no fill simulation is needed to filter
 * unfillable quotes. For each bidding solver it also records a full liquidity sweep — every
 * configured yield-vault token on every supported chain (raw ERC-20 + vault positions). Returns
 * `null` when no bid survives verification.
 *
 * `extractFill` decodes a bid's ERC-7821 calldata into the fill's order/output and `recoverSigner`
 * recovers its solver signature; both default to the viem implementations, but the indexer injects
 * VM2-safe variants (viem's keccak throws in the SubQuery sandbox).
 */
export async function aggregatePhantomBids(params: {
	nodeUrl: string
	/** RPC URL per supported EVM chain (stateMachineId -> url); must include the destination chain. */
	evmRpcUrls: Record<string, string>
	chain: string
	gatewayAddress: string
	commitment: string
	yieldVaults: YieldVaultMap
	/** SolverAccount on `chain` that our solvers delegate to; bids from anyone else are dropped. */
	solverAccount: string
	extractFill?: (callData: HexString, gatewayAddress: string) => FillData | null
	recoverSigner?: RecoverBidSigner
	bidNonceKey?: BidNonceKeyFn
	orderCommitment?: OrderCommitmentFn
	/**
	 * Balance reader shared across runs; defaults to a fresh per-run memo. Pass one built with
	 * `memoizedSolverBalance` when aggregating several same-block orders, and build it from the
	 * same `yieldVaults` passed here — the memo bakes in the vault map its balances include.
	 */
	getBalance?: SolverBalanceReader
	logger?: AggregationLogger
}): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrls, chain, gatewayAddress, commitment, yieldVaults, solverAccount, logger } = params
	const extractFill = params.extractFill ?? extractFillData
	const recoverSigner = params.recoverSigner ?? recoverBidSignerViem
	const bidNonceKey = params.bidNonceKey ?? CryptoUtils.bidNonceKey
	const orderCommitment = params.orderCommitment ?? orderCommitmentFromDecoded

	const destUrl = evmRpcUrls[chain]
	if (!destUrl) return null

	// Both bid checks need these, and neither can be skipped without letting an unverified quote into
	// the price, so a chain we can't resolve them for produces no snapshot at all. solverAccount is
	// typed as required but comes from a config lookup that can miss, so it is re-checked here.
	const chainId = evmChainId(chain)
	if (!solverAccount || chainId === null) {
		logger?.warn({ chain, commitment }, "Cannot verify phantom bids: no SolverAccount or chain id for chain")
		return null
	}

	const bids = await fetchBidsForOrder(nodeUrl, commitment)
	if (bids.length === 0) return null

	// Balances repeat across legs and the sweep; one memo serves the whole run.
	const getBalance = params.getBalance ?? memoizedSolverBalance(yieldVaults)

	// Quotes per leg, keyed by the leg's position in the order's asset lists. One bid carries a quote
	// for every leg its solver priced, so a solver that only handles some of the pairs still counts
	// towards those, and legs it skipped are left to the solvers that do handle them.
	const quotesByLeg = new Map<
		number,
		{
			outputToken: HexString
			quotes: { price: bigint; weight: bigint }[]
			bidders: PhantomLegBidder[]
		}
	>()
	const lpBalances: LpBalance[] = []
	// Bids are stored per substrate filler, but weight is a property of the EVM solver. Without this
	// one solver's bid, copied under N funded fillers, would count N times in the weighted median.
	const countedSolvers = new Set<string>()

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const solver = decoded.sender

			const fillData = extractFill(decoded.callData as HexString, gatewayAddress)
			if (!fillData) continue

			// The quoted price is read out of this order, so it must be the order being priced.
			const decodedCommitment = orderCommitment(fillData.order)
			if (!decodedCommitment || decodedCommitment.toLowerCase() !== commitment.toLowerCase()) {
				logger?.warn({ solver, commitment }, "Rejecting phantom bid: calldata order is not the indexed order")
				continue
			}

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const sessionKey = (fillData.order as any)?.session as HexString | undefined
			if (!sessionKey) {
				logger?.warn({ solver, commitment }, "Rejecting phantom bid: order carries no session key")
				continue
			}

			const verified = await isVerifiedSolverBid({
				userOp: decoded,
				commitment,
				sessionKey,
				chainId,
				solverAccount,
				evmRpcUrl: destUrl,
				recoverSigner,
				bidNonceKey,
				logger,
			})
			if (!verified) continue

			const normalizedSolver = solver.toLowerCase()
			if (countedSolvers.has(normalizedSolver)) {
				logger?.warn({ solver, commitment }, "Skipping phantom bid: solver already counted for this order")
				continue
			}
			countedSolvers.add(normalizedSolver)

			// The declaration rides in paymasterAndData, which the userOpHash covers, so it carries
			// the same authenticity as the quote itself.
			const acceptedSources = decodeAcceptedSourceChains(decoded.paymasterAndData)

			// A zero amount is how a solver declines a leg it does not price, so it is not a quote.
			// Weights are fetched concurrently: the memo caches promises, so identical output tokens
			// across legs still collapse to a single RPC round trip.
			const quotedLegs = [...fillData.legs.entries()].filter(([, leg]) => leg.solverAmount !== 0n)
			const weights = await Promise.all(
				// Price influence: the solver's liquidity in THIS leg's output token on the destination
				// chain, so a leg is weighted by the inventory that actually backs it.
				quotedLegs.map(([, leg]) => getBalance(destUrl, chain, toAddress(leg.outputToken), solver)),
			)
			for (const [position, [legIndex, leg]] of quotedLegs.entries()) {
				const weight = weights[position]
				const entry = quotesByLeg.get(legIndex) ?? { outputToken: leg.outputToken, quotes: [], bidders: [] }
				entry.quotes.push({ price: leg.solverAmount, weight })
				entry.bidders.push({ solver: normalizedSolver as HexString, weight, acceptedSources })
				quotesByLeg.set(legIndex, entry)
			}

			// Full liquidity picture: every configured token on every supported chain. Swept once per
			// bid rather than per leg, since it measures the solver's whole inventory either way.
			lpBalances.push(...(await sweepSolverLiquidity(evmRpcUrls, yieldVaults, solver, getBalance)))
		} catch (err) {
			logger?.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
		}
	}

	if (quotesByLeg.size === 0) return null

	// Each leg reports a single price: the liquidity-weighted median of the quotes for that leg.
	// lowestPrice and highestPrice carry that same value rather than the raw min/max of the bid set,
	// so consumers cannot read an outlier bid as if it were a tradeable bound.
	//
	// A quote's weight is the solver's inventory in THAT leg's output token on the destination
	// chain, so a zero-weight quote is one its solver cannot deliver at any price. Those are
	// dropped outright rather than merely down-weighted: they must not reach weightedMedian (with
	// nothing to weight by it picks a quote by position, letting whoever quotes the extreme set the
	// rate on zero capital), and they must not reach bidCount or `bidders`, where they would inflate
	// the solver count behind a price and mint zero-capacity PoolBidder/PoolRoute rows downstream.
	// A leg left with no backed quote at all is therefore absent entirely, exactly as if nobody had
	// quoted it — no snapshot, and its depth zeroes out downstream.
	const legs = [...quotesByLeg.entries()]
		.sort(([a], [b]) => a - b)
		.flatMap(([legIndex, { outputToken, quotes, bidders }]) => {
			// quotes and bidders are pushed in lockstep above, so the same predicate keeps them aligned.
			const backedQuotes = quotes.filter((quote) => quote.weight > 0n)
			const backedBidders = bidders.filter((bidder) => bidder.weight > 0n)
			if (backedQuotes.length === 0) {
				logger?.warn(
					{ commitment, chain, legIndex, outputToken, quotes: quotes.length },
					"Dropping phantom leg: no bidder holds the output token on this chain, so no quote is backed",
				)
				return []
			}

			const medianPrice = weightedMedian(backedQuotes)
			return [
				{
					legIndex,
					outputToken,
					lowestPrice: medianPrice,
					highestPrice: medianPrice,
					medianPrice,
					bidCount: backedQuotes.length,
					bidders: backedBidders,
				},
			]
		})

	return { legs, lpBalances }
}
