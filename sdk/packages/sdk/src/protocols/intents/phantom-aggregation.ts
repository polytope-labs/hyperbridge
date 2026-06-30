// Phantom-order price/liquidity aggregation. Lives in the SDK so the indexer (which persists the
// result as entities) and the simplex E2E test share one implementation. Bid decoding is injectable
// (extractFill) because the indexer must decode without viem — viem's @noble/hashes keccak throws
// "Uint8Array expected" in the SubQuery VM2 sandbox — so it passes a VM2-safe decoder; the default
// is the viem-based extractFillData, fine for Node consumers (tests, simplex).
import { decodeFunctionData } from "viem"
import { decodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { decodeUserOpScale } from "@/chains/intentsCoprocessor"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"

export type HexString = `0x${string}`

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

/** ERC-4626 vaults per chain, keyed by chain id then lowercase underlying token address. */
export type YieldVaultMap = Record<string, Record<string, string[]>>

export interface FillData {
	order: Record<string, unknown>
	options: Record<string, unknown>
	outputToken: HexString
	solverAmount: bigint
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

/** The aggregated result for a single phantom order's bid window. */
export interface PhantomAggregation {
	lowestPrice: bigint
	highestPrice: bigint
	medianPrice: bigint
	bidCount: number
	lpBalances: LpBalance[]
}

export interface AggregationLogger {
	warn: (payload: unknown, message: string) => void
}

// Liquidity-weighted median of solver quotes. Each quote's influence is proportional to `weight` —
// the solver's total balance for the output token across native + vault venues — so a solver that
// can actually deliver size moves the price more than one quoting on thin liquidity. Returns the
// lower weighted median: the smallest price whose cumulative weight reaches half of the total.
// Zero-weight quotes contribute nothing; if every weight is zero it falls back to the unweighted
// median so a price is still reported.
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
			const outputToken = (order as any)?.output?.assets?.[0]?.token as HexString | undefined
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputs = (options as any)?.outputs as { amount: bigint }[] | undefined
			if (!outputToken || !outputs?.length) continue
			return { order, options, outputToken, solverAmount: outputs[0].amount }
		} catch {
			continue
		}
	}
	return null
}

export async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
	const data = await rpcCall(nodeUrl, { id: 1, jsonrpc: "2.0", method: "intents_getBidsForOrder", params: [commitment] })
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

async function ethCallUint(evmRpcUrl: string, to: string, data: string): Promise<bigint> {
	try {
		const result = await rpcCall(evmRpcUrl, { id: 1, jsonrpc: "2.0", method: "eth_call", params: [{ to, data }, "latest"] })
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

// Sweeps a solver's liquidity for every configured yield-vault token on every supported chain: for
// each chain that has both an RPC (in evmRpcUrls) and configured tokens (in yieldVaults), the
// solver's balance (raw ERC-20 + ERC-4626 vault positions) for each token. Captures the LP's whole
// liquidity picture, not just the token of the bid being priced. Zero balances are skipped so the
// snapshot only records tokens the solver actually holds.
async function sweepSolverLiquidity(
	evmRpcUrls: Record<string, string>,
	yieldVaults: YieldVaultMap,
	solver: string,
): Promise<LpBalance[]> {
	const balances: LpBalance[] = []
	for (const [chain, tokens] of Object.entries(yieldVaults)) {
		const url = evmRpcUrls[chain]
		if (!url) continue
		for (const token of Object.keys(tokens)) {
			const balance = await getTotalSolverBalance(url, chain, token, solver, yieldVaults)
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
 * The liquidity-weighted median weights every quote by the solver's balance of the output token on
 * the destination chain, so a solver that can't actually deliver size carries little or no weight —
 * which is why no fill simulation is needed to filter unfillable quotes. For each bidding solver it
 * also records a full liquidity sweep — every configured yield-vault token on every supported chain
 * (raw ERC-20 + vault positions). Returns `null` when there are no decodable bids.
 *
 * `extractFill` decodes a bid's ERC-7821 calldata into the fill's order/output; it defaults to the
 * viem-based {@link extractFillData}, but the indexer injects a VM2-safe variant (viem's
 * decodeFunctionData keccak throws in the SubQuery sandbox).
 */
export async function aggregatePhantomBids(params: {
	nodeUrl: string
	/** RPC URL per supported EVM chain (stateMachineId -> url); must include the destination chain. */
	evmRpcUrls: Record<string, string>
	chain: string
	gatewayAddress: string
	commitment: string
	yieldVaults: YieldVaultMap
	extractFill?: (callData: HexString, gatewayAddress: string) => FillData | null
	logger?: AggregationLogger
}): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrls, chain, gatewayAddress, commitment, yieldVaults, logger } = params
	const extractFill = params.extractFill ?? extractFillData

	const destUrl = evmRpcUrls[chain]
	if (!destUrl) return null

	const bids = await fetchBidsForOrder(nodeUrl, commitment)
	if (bids.length === 0) return null

	const quotes: { price: bigint; weight: bigint }[] = []
	const lpBalances: LpBalance[] = []

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const solver = decoded.sender

			const fillData = extractFill(decoded.callData as HexString, gatewayAddress)
			if (!fillData) continue

			// Price influence: the solver's liquidity in the output token on the destination chain.
			const outputTokenAddress = toAddress(fillData.outputToken)
			const weight = await getTotalSolverBalance(destUrl, chain, outputTokenAddress, solver, yieldVaults)
			quotes.push({ price: fillData.solverAmount, weight })

			// Full liquidity picture: every configured token on every supported chain.
			lpBalances.push(...(await sweepSolverLiquidity(evmRpcUrls, yieldVaults, solver)))
		} catch (err) {
			logger?.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
		}
	}

	if (quotes.length === 0) return null

	const sortedPrices = quotes.map((q) => q.price).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))

	return {
		lowestPrice: sortedPrices[0],
		highestPrice: sortedPrices[sortedPrices.length - 1],
		medianPrice: weightedMedian(quotes),
		bidCount: quotes.length,
		lpBalances,
	}
}

