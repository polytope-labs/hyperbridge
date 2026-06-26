// Phantom-order price/liquidity aggregation. Lives in the SDK so both the indexer (which persists
// the result as entities) and integration tests (which assert it) share one implementation. The two
// pieces of indexer-specific config — per-token ERC-20 storage slots and the ERC-4626 vault map —
// are passed in, keeping this module free of indexer-generated data.
import { decodeFunctionData, encodeFunctionData, encodeAbiParameters, keccak256, concat, toHex } from "viem"
import { decodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { decodeUserOpScale } from "@/chains/intentsCoprocessor"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"

export type HexString = `0x${string}`

export const FILL_ORDER_ABI = IntentGatewayV2.ABI

// topic0 of OrderFilled(bytes32,address,TokenInfo[],TokenInfo[]); its presence in the simulated call
// logs is what tells us the fill actually went through rather than just not reverting.
export const ORDER_FILLED_TOPIC = keccak256(
	toHex("OrderFilled(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])"),
).toLowerCase()

// A deadline far beyond any real chain head so the simulated order clears the gateway's
// `deadline < block.number` expiry check. The on-chain phantom order keeps its own expired deadline.
export const SIM_DEADLINE = 1n << 48n

export interface TokenSlots {
	balanceSlot: bigint
	allowanceSlot: bigint
}
/** Per-token ERC-20 storage slots, keyed by lowercase token address. */
export type TokenSlotOverrides = Record<string, TokenSlots>
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

const NOOP_LOGGER: AggregationLogger = { warn: () => {} }

// Strips a bytes32 token field to a 20-byte lowercase address (or normalises an address as-is).
// Inlined rather than imported from the SDK's util barrel so the intents-helpers entry stays light.
function toAddress(token: string): HexString {
	const hex = token.toLowerCase().replace(/^0x/, "")
	const addr = hex.length > 40 ? hex.slice(-40) : hex.padStart(40, "0")
	return `0x${addr}` as HexString
}

export function tokenSlots(address: string, overrides: TokenSlotOverrides): TokenSlots {
	return overrides[address.toLowerCase()] ?? { balanceSlot: 0n, allowanceSlot: 1n }
}

// Whether a token has a configured slot override. Tokens without one fall back to the OZ default
// (0/1), which is wrong for most real tokens, so the caller warns when this returns false.
export function hasTokenSlotOverride(address: string, overrides: TokenSlotOverrides): boolean {
	return address.toLowerCase() in overrides
}

// _orders is mapping(bytes32 => mapping(address => uint256)) at slot 9 in the IntentGateway.
// (PR #988 removed the _admin slot, shifting _orders down from slot 10 to slot 9.)
// The inner mapping is keyed by `address`, so the key must be the token left-padded to 32 bytes
// (abi.encode(address)). `inputToken` may be a 20-byte address or a 32-byte token field; normalise
// both to the address-as-uint256 form before hashing.
export function ordersStorageSlot(commitment: HexString, inputToken: HexString): HexString {
	const tokenKey = toHex(BigInt(inputToken), { size: 32 })
	const innerSlot = keccak256(concat([commitment, toHex(9n, { size: 32 })]))
	return keccak256(concat([tokenKey, innerSlot]))
}

export function erc20BalanceSlot(holder: HexString, slot: bigint): HexString {
	return keccak256(concat([toHex(BigInt(holder), { size: 32 }), toHex(slot, { size: 32 })]))
}

// _allowances[owner][spender]: inner slot keys on owner, outer on spender.
export function erc20AllowanceSlot(owner: HexString, spender: HexString, slot: bigint): HexString {
	const innerSlot = keccak256(concat([toHex(BigInt(owner), { size: 32 }), toHex(slot, { size: 32 })]))
	return keccak256(concat([toHex(BigInt(spender), { size: 32 }), innerSlot]))
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

// Rebuilds the bid's order for simulation. Matching source to destination routes the gateway through
// _fillSameChain (no ISMP dispatch), the future deadline clears the expiry check, and pointing the
// single output at the solver for solverAmount makes _fillSameChain run safeTransferFrom(solver ->
// solver) and read the injected escrow, which is the liquidity we want to validate. The phantom
// order's output amount is zero, so without this the fill is a no-op. Session is left as decoded
// (already the zero address); solver selection is disabled via a storage override on the gateway.
export function buildSimulationOrder(
	order: Record<string, unknown>,
	solver: string,
	solverAmount: bigint,
): Record<string, unknown> {
	const outputInfo = order.output as {
		beneficiary: HexString
		assets: { token: HexString; amount: bigint }[]
		call: HexString
	}
	return {
		...order,
		source: order.destination,
		deadline: SIM_DEADLINE,
		output: {
			...outputInfo,
			beneficiary: toHex(BigInt(solver), { size: 32 }),
			assets: outputInfo.assets.map((asset, i) => ({
				...asset,
				amount: i === 0 ? solverAmount : asset.amount,
			})),
		},
	}
}

// Simulates a solver's fill via eth_simulateV1 and returns true only if it succeeds AND emits
// OrderFilled.
async function simulateBid(
	evmRpcUrl: string,
	solver: string,
	fillData: FillData,
	gatewayAddress: string,
	inputToken: HexString,
	inputAmount: bigint,
	overrides: TokenSlotOverrides,
	logger: AggregationLogger,
): Promise<boolean> {
	try {
		const { order, options, outputToken, solverAmount } = fillData

		const modifiedOrder = buildSimulationOrder(order, solver, solverAmount)

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const orderType = (FILL_ORDER_ABI as unknown as any[]).find((f) => f.name === "fillOrder")?.inputs?.[0]
		if (!orderType) return false

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const newCommitment = keccak256((encodeAbiParameters as any)([orderType], [modifiedOrder])) as HexString

		const outputTokenAddress = toAddress(outputToken)
		const inputTokenAddress = toAddress(inputToken)
		for (const token of [inputTokenAddress, outputTokenAddress]) {
			if (!hasTokenSlotOverride(token, overrides)) {
				logger.warn(
					{ token },
					"No token slot override; assuming OZ default slots 0/1, simulation may be inaccurate",
				)
			}
		}
		const inputSlots = tokenSlots(inputTokenAddress, overrides)
		const outputSlots = tokenSlots(outputTokenAddress, overrides)

		const stateDiff = {
			[gatewayAddress]: {
				stateDiff: {
					[ordersStorageSlot(newCommitment, inputToken)]: toHex(inputAmount, { size: 32 }),
					[toHex(5n, { size: 32 })]: toHex(0n, { size: 32 }),
					[toHex(8n, { size: 32 })]: toHex(0n, { size: 32 }),
				},
			},
			[inputTokenAddress]: {
				stateDiff: {
					[erc20BalanceSlot(gatewayAddress as HexString, inputSlots.balanceSlot)]: toHex(inputAmount, {
						size: 32,
					}),
				},
			},
			[outputTokenAddress]: {
				stateDiff: {
					[erc20AllowanceSlot(solver as HexString, gatewayAddress as HexString, outputSlots.allowanceSlot)]:
						toHex(solverAmount, { size: 32 }),
				},
			},
		}

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const callData = (encodeFunctionData as any)({
			abi: FILL_ORDER_ABI,
			functionName: "fillOrder",
			args: [modifiedOrder, options],
		})

		const response = await fetch(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_simulateV1",
				params: [
					{
						blockStateCalls: [{ stateOverrides: stateDiff, calls: [{ from: solver, to: gatewayAddress, data: callData }] }],
						validation: false,
						traceTransfers: false,
					},
					"latest",
				],
			}),
		})
		const result = await response.json()
		if (result.error) return false

		const call = result.result?.[0]?.calls?.[0]
		if (!call || call.status !== "0x1") return false

		const logs: { topics?: string[] }[] = call.logs ?? []
		return logs.some((log) => log.topics?.[0]?.toLowerCase() === ORDER_FILLED_TOPIC)
	} catch {
		return false
	}
}

export async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
	const response = await fetch(nodeUrl, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "intents_getBidsForOrder", params: [commitment] }),
	})
	const data = await response.json()
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

async function ethCallUint(evmRpcUrl: string, to: string, data: string): Promise<bigint> {
	try {
		const response = await fetch(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "eth_call", params: [{ to, data }, "latest"] }),
		})
		const result = await response.json()
		if (result.error || !result.result || result.result === "0x") return 0n
		return BigInt(result.result)
	} catch {
		return 0n
	}
}

// Sums the solver's redeemable balance of a single token on its destination chain: the raw ERC-20
// balance plus any ERC-4626 vault positions wrapping it.
export async function getTotalSolverBalance(
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
// liquidity picture, not just the token of the bid being priced.
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
			balances.push({ solver, chain, tokenAddress: token as HexString, balance })
		}
	}
	return balances
}

/**
 * Aggregates every bid for a phantom order into a single price/liquidity snapshot.
 *
 * Fetches the live bids via `intents_getBidsForOrder` and simulates each filler's fillOrder against
 * the destination chain to confirm it would succeed. The liquidity-weighted median weights each
 * quote by the solver's balance of the output token on the destination chain. For each bidding
 * solver it also records a full liquidity sweep — every configured yield-vault token on every
 * supported chain (raw ERC-20 + vault positions). Returns `null` when no bid passes simulation.
 */
export async function aggregatePhantomBids(params: {
	nodeUrl: string
	/** RPC URL per supported EVM chain (stateMachineId -> url); must include the destination chain. */
	evmRpcUrls: Record<string, string>
	chain: string
	gatewayAddress: string
	commitment: string
	/** Phantom input token (tokenA), as a 20-byte address or 32-byte token field. */
	inputToken: HexString
	standardAmount: bigint
	tokenSlotOverrides: TokenSlotOverrides
	yieldVaults: YieldVaultMap
	logger?: AggregationLogger
}): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrls, chain, gatewayAddress, commitment, inputToken, standardAmount } = params
	const { tokenSlotOverrides, yieldVaults } = params
	const logger = params.logger ?? NOOP_LOGGER

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

			const fillData = extractFillData(decoded.callData as HexString, gatewayAddress)
			if (!fillData) continue

			const simOk = await simulateBid(
				destUrl,
				solver,
				fillData,
				gatewayAddress,
				inputToken,
				standardAmount,
				tokenSlotOverrides,
				logger,
			)
			if (!simOk) continue

			// Price influence: the solver's liquidity in the output token on the destination chain.
			const outputTokenAddress = toAddress(fillData.outputToken)
			const weight = await getTotalSolverBalance(destUrl, chain, outputTokenAddress, solver, yieldVaults)
			quotes.push({ price: fillData.solverAmount, weight })

			// Full liquidity picture: every configured token on every supported chain.
			lpBalances.push(...(await sweepSolverLiquidity(evmRpcUrls, yieldVaults, solver)))
		} catch (err) {
			logger.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
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
