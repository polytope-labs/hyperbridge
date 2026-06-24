import { SubstrateBlock } from "@subql/types"
import { decodeFunctionData, encodeFunctionData, encodeAbiParameters, keccak256, concat, toHex } from "viem"
import { decodeERC7821ExecuteBatch, decodeUserOpScale, IntentGatewayV2 } from "@hyperbridge/sdk/intents-helpers"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/intent-gateway-v2-addresses"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { TOKEN_EQUIVALENCES } from "@/token-equivalences"
import { getActiveIntervalCommitments } from "@/active-phantom-interval"
import { PhantomOrder, PhantomOrderLpBalance, PhantomOrderPriceSnapshot } from "@/configs/src/types"

type HexString = `0x${string}`

const FILL_ORDER_ABI = IntentGatewayV2.ABI

// _orders is mapping(bytes32 => mapping(address => uint256)) at slot 10.
// inputTokenBytes32 must be the address left-padded to 32 bytes, matching abi.encode(address).
function ordersStorageSlot(commitment: HexString, inputTokenBytes32: HexString): HexString {
	const ORDERS_SLOT = BigInt(10)
	const innerSlot = keccak256(concat([commitment, toHex(ORDERS_SLOT, { size: 32 })]))
	return keccak256(concat([inputTokenBytes32, innerSlot]))
}

interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

interface FillData {
	order: Record<string, unknown>
	options: { relayerFee: bigint; nativeDispatchFee: bigint; outputs: { token: HexString; amount: bigint }[] }
	outputToken: HexString
	solverAmount: bigint
}

function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			const decoded = decodeFunctionData({ abi: FILL_ORDER_ABI, data: call.data as HexString })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const order = decoded.args[0] as unknown as Record<string, unknown>
			const options = decoded.args[1] as unknown as {
				relayerFee: bigint
				nativeDispatchFee: bigint
				outputs: { token: HexString; amount: bigint }[]
			}
			if (!options.outputs?.length) continue
			const outputToken = (order.output as { assets: { token: HexString }[] }).assets?.[0]?.token
			if (!outputToken) continue
			return { order, options, outputToken, solverAmount: options.outputs[0].amount }
		} catch {
			continue
		}
	}
	return null
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

// Verifies the solver can deliver solverAmount of the output token by simulating fillOrder via eth_call.
// The order is modified to force same-chain routing (source = destination) and set the solver as
// beneficiary, so safeTransferFrom(solver → solver) validates their balance and allowance in place.
// The commitment is recomputed from the modified order and all required storage slots are injected
// via stateDiff. Block number is overridden to 0 so deadline checks always pass.
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

		const inputTokenAddress = bytes32ToBytes20(inputTokenBytes32)
		const outputTokenAddress = bytes32ToBytes20(outputToken)
		const solverPadded = toHex(BigInt(solver), { size: 32 })
		const MAX_UINT256 = 2n ** 256n - 1n

		// OZ ERC-20 storage layout: _balances at slot 0, _allowances at slot 1.
		const balanceSlot = (owner: string): HexString =>
			keccak256(concat([toHex(BigInt(owner), { size: 32 }), toHex(0n, { size: 32 })]))
		const allowanceSlot = (owner: string, spender: string): HexString => {
			const inner = keccak256(concat([toHex(BigInt(owner), { size: 32 }), toHex(1n, { size: 32 })]))
			return keccak256(concat([toHex(BigInt(spender), { size: 32 }), inner]))
		}

		const outputInfo = order.output as {
			beneficiary: HexString
			assets: { token: HexString; amount: bigint }[]
			call: HexString
		}

		const modifiedOrder = {
			...order,
			source: (order as { destination: HexString }).destination,
			session: "0x0000000000000000000000000000000000000000",
			output: {
				...outputInfo,
				beneficiary: solverPadded,
				assets: outputInfo.assets.map((asset, i) => ({
					...asset,
					amount: i === 0 ? solverAmount : asset.amount,
				})),
				call: "0x",
			},
		}

		const fillOrderAbi = FILL_ORDER_ABI.find(
			(item): item is typeof item & { type: "function"; name: string; inputs: readonly unknown[] } =>
				item.type === "function" && "name" in item && item.name === "fillOrder",
		)
		if (!fillOrderAbi) return false

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const newFillCalldata = (encodeFunctionData as any)({
			abi: FILL_ORDER_ABI,
			functionName: "fillOrder",
			args: [modifiedOrder, options],
		}) as HexString

		const orderAbiType = (fillOrderAbi as { inputs: readonly unknown[] }).inputs[0]
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const newCommitment = keccak256((encodeAbiParameters as any)([orderAbiType], [modifiedOrder])) as HexString

		const escrowSlot = ordersStorageSlot(newCommitment, inputTokenBytes32)

		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [
					{ from: solver, to: gatewayAddress, data: newFillCalldata },
					"latest",
					{
						[gatewayAddress]: {
							stateDiff: {
								// slot 5 holds dispatcher + solverSelection — zeroing both avoids the tload selection check
								[toHex(5n, { size: 32 })]: toHex(0n, { size: 32 }),
								// slot 8 is priceOracle — zero skips the recordSpread call
								[toHex(8n, { size: 32 })]: toHex(0n, { size: 32 }),
								[escrowSlot]: toHex(inputAmount, { size: 32 }),
							},
						},
						[inputTokenAddress]: {
							stateDiff: {
								// gateway needs a balance so _withdraw's safeTransfer to the solver succeeds
								[balanceSlot(gatewayAddress)]: toHex(inputAmount, { size: 32 }),
							},
						},
						[outputTokenAddress]: {
							stateDiff: {
								// the batch-level ERC-20 approve is dropped when extractFillData unwraps the inner call
								[allowanceSlot(solver, gatewayAddress)]: toHex(MAX_UINT256, { size: 32 }),
							},
						},
					},
					{ number: toHex(0n) },
				],
			}),
		})
		const result = await response.json()
		return !result.error
	} catch {
		return false
	}
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

async function getTotalSolverBalance(
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

// Uses TOKEN_EQUIVALENCES to resolve the per-chain address for the output token before summing.
// Chains with no mapping are skipped rather than queried with the wrong address.
async function getTotalSolverBalanceAllChains(
	destinationChain: string,
	outputTokenAddress: string,
	solver: string,
): Promise<bigint> {
	const tokenLower = outputTokenAddress.toLowerCase()
	const chainMap = TOKEN_EQUIVALENCES[destinationChain]?.[tokenLower] ?? {}

	const checks = Object.entries(ENV_CONFIG)
		.filter(([chain]) => chain.startsWith("EVM-"))
		.map(async ([chain, url]) => {
			const evmRpcUrl = replaceWebsocketWithHttp(url ?? "")
			if (!evmRpcUrl) return 0n
			const tokenAddress = chain === destinationChain ? outputTokenAddress : chainMap[chain]
			if (!tokenAddress) return 0n
			return getTotalSolverBalance(evmRpcUrl, chain, tokenAddress, solver)
		})
	const balances = await Promise.all(checks)
	return balances.reduce((acc, b) => acc + b, 0n)
}

async function getActiveCommitment(blockHash: string): Promise<string | null> {
	try {
		const storageKey = api.query.intentsCoprocessor.currentPhantomOrder.key()
		const rawResult = await api.rpc.state.getStorage(storageKey, blockHash)
		const hex: string = rawResult.toHex()
		if (!hex || hex === "0x") return null
		const bare = hex.replace("0x", "")
		if (bare.length < 64) return null
		return "0x" + bare.slice(0, 64)
	} catch {
		return null
	}
}

// Returns the upper-middle element for even-length arrays rather than averaging the two midpoints.
function medianOf(values: bigint[]): bigint {
	const sorted = [...values].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))
	return sorted[Math.floor(sorted.length / 2)]
}

export const handlePhantomOrderPrices = wrap(async (event: SubstrateBlock): Promise<void> => {
	const blockNumber = event.block.header.number.toBigInt()
	const blockHash = event.block.header.hash.toString()

	// Use the in-session set when available — it covers every pair in the interval, not just the
	// last one written to currentPhantomOrder. Falls back to chain-state after a restart.
	const { commitments: activeCommitments } = getActiveIntervalCommitments()

	let phantomOrders: PhantomOrder[]
	if (activeCommitments.size > 0) {
		phantomOrders = (
			await Promise.all([...activeCommitments].map((c) => PhantomOrder.get(c)))
		).filter((o): o is PhantomOrder => o !== null)
	} else {
		const activeCommitment = await getActiveCommitment(blockHash)
		if (!activeCommitment) return
		const anchor = await PhantomOrder.get(activeCommitment)
		if (!anchor) return
		phantomOrders = await PhantomOrder.getByCreatedAtBlock(anchor.createdAtBlock, { limit: 100 })
	}

	if (!phantomOrders.length) return

	const host = getHostStateMachine(chainId)
	const nodeUrl = replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "")
	if (!nodeUrl) {
		logger.warn({ host }, "No RPC URL configured for Hyperbridge node")
		return
	}

	const blockTimestamp = await getBlockTimestamp(blockHash, host)

	for (const phantom of phantomOrders) {
		const snapshotId = `${phantom.id}-${blockNumber}`
		if (await PhantomOrderPriceSnapshot.get(snapshotId)) continue

		let bids: RpcBidInfo[]
		try {
			bids = await fetchBidsForOrder(nodeUrl, phantom.id)
		} catch (err) {
			logger.warn({ err, commitment: phantom.id }, "intents_getBidsForOrder failed")
			continue
		}

		if (bids.length === 0) continue

		const evmUrl = replaceWebsocketWithHttp(ENV_CONFIG[phantom.chain] ?? "")
		const gatewayAddress = INTENT_GATEWAY_V2_ADDRESSES[phantom.chain as keyof typeof INTENT_GATEWAY_V2_ADDRESSES]
		if (!evmUrl || !gatewayAddress) continue

		const prices: bigint[] = []
		let bestLpBalance = 0n

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
					BigInt(phantom.standardAmount.toString()),
				)
				if (!simOk) continue

				const outputTokenAddress = bytes32ToBytes20(fillData.outputToken)
				const totalBalance = await getTotalSolverBalanceAllChains(phantom.chain, outputTokenAddress, solver)

				prices.push(fillData.solverAmount)
				if (totalBalance > bestLpBalance) bestLpBalance = totalBalance

				await PhantomOrderLpBalance.create({
					id: `${phantom.id}-${blockNumber}-${solver}`,
					commitment: phantom.id,
					blockNumber,
					solver,
					balance: totalBalance,
					snapshotTime: timestampToDate(blockTimestamp),
				}).save()
			} catch (err) {
				logger.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
			}
		}

		if (prices.length === 0) continue

		const sorted = [...prices].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))

		await PhantomOrderPriceSnapshot.create({
			id: snapshotId,
			commitment: phantom.id,
			blockNumber,
			lowestPrice: sorted[0],
			highestPrice: sorted[sorted.length - 1],
			medianPrice: medianOf(prices),
			bidCount: prices.length,
			lpBalance: bestLpBalance > 0n ? bestLpBalance : undefined,
			snapshotTime: timestampToDate(blockTimestamp),
		}).save()

		logger.info({ commitment: phantom.id, blockNumber, bidCount: prices.length }, "PhantomOrderPriceSnapshot saved")
	}
})
