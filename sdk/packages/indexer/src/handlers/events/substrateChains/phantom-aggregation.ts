// Core phantom-order price/liquidity aggregation, factored out of the SubQuery handler so it can be
// exercised directly in integration tests (against a live simnode + a forked EVM node) without the
// SubQuery runtime. The handler is a thin wrapper that calls aggregatePhantomBids() and persists the
// result as entities; the logic that matters — fetching bids, simulating each fill, measuring solver
// liquidity, and computing the liquidity-weighted median — lives here.
import { encodeFunctionData, encodeAbiParameters, keccak256, toHex } from "viem"
import { decodeUserOpScale } from "@hyperbridge/sdk/intents-helpers"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import {
	buildSimulationOrder,
	erc20AllowanceSlot,
	erc20BalanceSlot,
	extractFillData,
	FILL_ORDER_ABI,
	FillData,
	hasTokenSlotOverride,
	HexString,
	ordersStorageSlot,
	ORDER_FILLED_TOPIC,
	tokenSlots,
	weightedMedian,
} from "./phantom-simulation.helpers"

export interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

/** One solver's measured liquidity for the output token at this snapshot. */
export interface LpBalance {
	solver: string
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

/** Minimal logger so this module doesn't depend on the SubQuery global. */
export interface AggregationLogger {
	warn: (payload: unknown, message: string) => void
}

const NOOP_LOGGER: AggregationLogger = { warn: () => {} }

// Simulates a solver's fill via eth_simulateV1 and returns true only if it succeeds AND emits
// OrderFilled. See buildSimulationOrder for why the order is rewritten and which slots are injected.
async function simulateBid(
	evmRpcUrl: string,
	solver: string,
	fillData: FillData,
	gatewayAddress: string,
	inputTokenBytes32: HexString,
	inputAmount: bigint,
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

		const outputTokenAddress = bytes32ToBytes20(outputToken) as HexString
		const inputTokenAddress = bytes32ToBytes20(inputTokenBytes32) as HexString
		for (const token of [inputTokenAddress, outputTokenAddress]) {
			if (!hasTokenSlotOverride(token)) {
				logger.warn(
					{ token },
					"No TOKEN_SLOT_OVERRIDES entry; assuming OZ default slots 0/1, simulation may be inaccurate",
				)
			}
		}
		const inputSlots = tokenSlots(inputTokenAddress)
		const outputSlots = tokenSlots(outputTokenAddress)

		const stateDiff = {
			[gatewayAddress]: {
				stateDiff: {
					[ordersStorageSlot(newCommitment, inputTokenBytes32)]: toHex(inputAmount, { size: 32 }),
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

		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_simulateV1",
				params: [
					{
						blockStateCalls: [
							{
								stateOverrides: stateDiff,
								calls: [{ from: solver, to: gatewayAddress, data: callData }],
							},
						],
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
	const response = await fetchWithRetry(nodeUrl, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "intents_getBidsForOrder", params: [commitment] }),
	})
	const data = await response.json()
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

async function getTokenBalance(evmRpcUrl: string, token: string, holder: string): Promise<bigint> {
	try {
		const paddedHolder = holder.replace("0x", "").padStart(64, "0")
		const data = `0x70a08231${paddedHolder}` as HexString
		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "eth_call", params: [{ to: token, data }, "latest"] }),
		})
		const result = await response.json()
		if (result.error || !result.result || result.result === "0x") return 0n
		return BigInt(result.result)
	} catch {
		return 0n
	}
}

async function getVaultBalance(evmRpcUrl: string, vault: string, owner: string): Promise<bigint> {
	try {
		const paddedOwner = owner.replace("0x", "").padStart(64, "0")
		// maxWithdraw(address) selector
		const data = `0xce96cb77${paddedOwner}` as HexString
		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "eth_call", params: [{ to: vault, data }, "latest"] }),
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
): Promise<bigint> {
	const raw = await getTokenBalance(evmRpcUrl, token, solver)
	const vaultMap = YIELD_VAULT_ADDRESSES[chain] ?? {}
	const vaults = vaultMap[token.toLowerCase()] ?? []
	const vaultBalances = await Promise.all(vaults.map((v) => getVaultBalance(evmRpcUrl, v, solver)))
	return vaultBalances.reduce((acc, b) => acc + b, raw)
}

/**
 * Aggregates every bid for a phantom order into a single price/liquidity snapshot.
 *
 * Fetches the live bids via `intents_getBidsForOrder`, simulates each filler's fillOrder against the
 * (forked) EVM chain to confirm it would succeed, measures the solver's liquidity for the output
 * token, and returns the quote range plus the liquidity-weighted median. Returns `null` when no bid
 * passes simulation, so the caller can skip writing an empty snapshot.
 */
export async function aggregatePhantomBids(params: {
	nodeUrl: string
	evmRpcUrl: string
	chain: string
	gatewayAddress: string
	commitment: string
	/** Phantom input token (tokenA), as stored on the order. */
	inputToken: HexString
	standardAmount: bigint
	logger?: AggregationLogger
}): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrl, chain, gatewayAddress, commitment, inputToken, standardAmount } = params
	const logger = params.logger ?? NOOP_LOGGER

	const bids = await fetchBidsForOrder(nodeUrl, commitment)
	if (bids.length === 0) return null

	const quotes: { price: bigint; weight: bigint }[] = []
	const lpBalances: LpBalance[] = []

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const callData = decoded.callData as HexString
			const solver = decoded.sender

			const fillData = extractFillData(callData, gatewayAddress)
			if (!fillData) continue

			const simOk = await simulateBid(
				evmRpcUrl,
				solver,
				fillData,
				gatewayAddress,
				inputToken,
				standardAmount,
				logger,
			)
			if (!simOk) continue

			const outputTokenAddress = bytes32ToBytes20(fillData.outputToken) as HexString
			const balance = await getTotalSolverBalance(evmRpcUrl, chain, outputTokenAddress, solver)

			quotes.push({ price: fillData.solverAmount, weight: balance })
			lpBalances.push({ solver, tokenAddress: outputTokenAddress, balance })
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
