import { SubstrateEvent } from "@subql/types"
import { encodeFunctionData, encodeAbiParameters, keccak256, toHex } from "viem"
import { decodeUserOpScale } from "@hyperbridge/sdk/intents-helpers"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/intent-gateway-v2-addresses"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { PhantomOrder, PhantomOrderLpBalance, PhantomOrderPriceSnapshot } from "@/configs/src/types"
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
} from "./phantom-simulation.helpers"

interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

async function simulateBid(
	evmRpcUrl: string,
	solver: string,
	fillData: FillData,
	gatewayAddress: string,
	inputTokenBytes32: HexString,
	inputAmount: bigint,
): Promise<boolean> {
	try {
		const { order, options, outputToken, solverAmount } = fillData

		// Rebuild the order so the gateway routes through _fillSameChain and actually runs the
		// transfer whose balance and allowance we override below; see buildSimulationOrder.
		const modifiedOrder = buildSimulationOrder(order, solver, solverAmount)

		// Recompute the commitment for the modified order so the escrow injection keys correctly.
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
					// Phantom escrow: _orders[newCommitment][inputToken] = inputAmount
					[ordersStorageSlot(newCommitment, inputTokenBytes32)]: toHex(inputAmount, { size: 32 }),
					// Disable solver selection and price oracle
					[toHex(5n, { size: 32 })]: toHex(0n, { size: 32 }),
					[toHex(8n, { size: 32 })]: toHex(0n, { size: 32 }),
				},
			},
			[inputTokenAddress]: {
				stateDiff: {
					// Gateway must hold inputAmount to release to the solver via _withdraw.
					[erc20BalanceSlot(gatewayAddress as HexString, inputSlots.balanceSlot)]: toHex(inputAmount, {
						size: 32,
					}),
				},
			},
			[outputTokenAddress]: {
				stateDiff: {
					// Solver must have approved the gateway to pull solverAmount of the output token.
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

		// eth_simulateV1 (not eth_call) so we get the emitted logs back and can confirm OrderFilled.
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

async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
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
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [{ to: token, data }, "latest"],
			}),
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
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [{ to: vault, data }, "latest"],
			}),
		})
		const result = await response.json()
		if (result.error || !result.result || result.result === "0x") return 0n
		return BigInt(result.result)
	} catch {
		return 0n
	}
}

// Sums the solver's redeemable balance of a single token on its destination chain: the raw
// ERC-20 balance plus any ERC-4626 vault positions wrapping it. This is the liquidity that
// matters at fill time, and it stays per-token so balances of different tokens are never mixed.
async function getTotalSolverBalance(evmRpcUrl: string, chain: string, token: string, solver: string): Promise<bigint> {
	const raw = await getTokenBalance(evmRpcUrl, token, solver)
	const vaultMap = YIELD_VAULT_ADDRESSES[chain] ?? {}
	const vaults = vaultMap[token.toLowerCase()] ?? []
	const vaultBalances = await Promise.all(vaults.map((v) => getVaultBalance(evmRpcUrl, v, solver)))
	return vaultBalances.reduce((acc, b) => acc + b, raw)
}

// Returns the upper-middle element for even-length arrays rather than averaging the two midpoints.
function medianOf(values: bigint[]): bigint {
	const sorted = [...values].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))
	return sorted[Math.floor(sorted.length / 2)]
}

// Triggered by PhantomBidWindowExhausted once a phantom order's bid window closes, so every bid is
// already in. Aggregates that single order's bids into one price snapshot.
export const handlePhantomOrderPrices = wrap(async (event: SubstrateEvent): Promise<void> => {
	const blockNumber = event.block.block.header.number.toBigInt()
	const blockHash = event.block.block.header.hash.toString()

	const [commitmentData] = event.event.data
	const commitment = commitmentData.toHex()

	const phantom = await PhantomOrder.get(commitment)
	if (!phantom) return

	const snapshotId = `${commitment}-${blockNumber}`
	if (await PhantomOrderPriceSnapshot.get(snapshotId)) return

	const host = getHostStateMachine(chainId)
	const nodeUrl = replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "")
	if (!nodeUrl) {
		logger.warn({ host }, "No RPC URL configured for Hyperbridge node")
		return
	}

	let bids: RpcBidInfo[]
	try {
		bids = await fetchBidsForOrder(nodeUrl, commitment)
	} catch (err) {
		logger.warn({ err, commitment }, "intents_getBidsForOrder failed")
		return
	}
	if (bids.length === 0) return

	const evmUrl = replaceWebsocketWithHttp(ENV_CONFIG[phantom.chain] ?? "")
	const gatewayAddress = INTENT_GATEWAY_V2_ADDRESSES[phantom.chain as keyof typeof INTENT_GATEWAY_V2_ADDRESSES]
	if (!evmUrl || !gatewayAddress) return

	const blockTimestamp = await getBlockTimestamp(blockHash, host)
	const prices: bigint[] = []

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const callData = decoded.callData as HexString
			const solver = decoded.sender

			const fillData = extractFillData(callData, gatewayAddress)
			if (!fillData) continue

			const simOk = await simulateBid(
				evmUrl,
				solver,
				fillData,
				gatewayAddress,
				phantom.tokenA as HexString,
				phantom.standardAmount,
			)
			if (!simOk) continue

			const outputTokenAddress = bytes32ToBytes20(fillData.outputToken)
			const totalBalance = await getTotalSolverBalance(evmUrl, phantom.chain, outputTokenAddress, solver)

			prices.push(fillData.solverAmount)

			await PhantomOrderLpBalance.create({
				id: `${commitment}-${blockNumber}-${solver}`,
				commitment,
				blockNumber,
				solver,
				tokenAddress: outputTokenAddress,
				balance: totalBalance,
				snapshotTime: timestampToDate(blockTimestamp),
			}).save()
		} catch (err) {
			logger.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
		}
	}

	if (prices.length === 0) return

	const sorted = [...prices].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))

	await PhantomOrderPriceSnapshot.create({
		id: snapshotId,
		commitment,
		blockNumber,
		lowestPrice: sorted[0],
		highestPrice: sorted[sorted.length - 1],
		medianPrice: medianOf(prices),
		bidCount: prices.length,
		snapshotTime: timestampToDate(blockTimestamp),
	}).save()

	logger.info({ commitment, blockNumber, bidCount: prices.length }, "PhantomOrderPriceSnapshot saved")
})
