import { SubstrateEvent } from "@subql/types"
import { decodeFunctionData, toHex } from "viem"
import {
	decodeUserOpScale,
	decodeERC7821ExecuteBatch,
	IntentGatewayABI,
	requestCommitmentKey,
	type HexString,
} from "@hyperbridge/sdk"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/intent-gateway-v2-addresses"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { PhantomOrder, PhantomOrderPriceSnapshot } from "@/configs/src/types"

// ─── Types ───────────────────────────────────────────────────────────────────

interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

interface FillData {
	outputs: { token: string; amount: bigint }[]
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			const decoded = decodeFunctionData({ abi: IntentGatewayABI, data: call.data })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const options = decoded.args[1] as { outputs: { token: string; amount: bigint }[] }
			if (!options.outputs?.length) continue
			return { outputs: options.outputs.map((o) => ({ token: o.token as string, amount: o.amount })) }
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

// Block number is overridden to 0 so the deadline check (deadline < block.number)
// always passes. The intent gateway's requestCommitments slot is overridden so the
// gateway treats the phantom commitment as a registered order.
async function simulateBid(
	evmRpcUrl: string,
	solver: string,
	callData: HexString,
	gatewayAddress: string,
	commitment: string,
): Promise<boolean> {
	try {
		const { slot1, slot2 } = requestCommitmentKey(commitment as HexString)
		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [
					{ from: solver, to: solver, data: callData },
					"latest",
					{
						[gatewayAddress]: {
							stateDiff: {
								[slot2]: toHex(1n, { size: 32 }),
								[slot1]: toHex(1n, { size: 32 }),
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

// Calls ERC-4626 maxWithdraw(owner) to get the solver's redeemable balance from a vault.
async function getVaultBalance(evmRpcUrl: string, vault: string, owner: string): Promise<bigint> {
	try {
		const paddedOwner = owner.replace("0x", "").padStart(64, "0")
		// maxWithdraw(address owner) → bytes4 selector ce96cb77
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

// Returns raw ERC-20 balance plus total redeemable from all configured yield vaults
// for the given token on the given chain.
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

// Reads the commitment of the currently active phantom order from storage.
// The first 32 bytes of the SCALE-encoded value are always the H256 commitment.
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

function medianOf(values: bigint[]): bigint {
	const sorted = [...values].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))
	return sorted[Math.floor(sorted.length / 2)]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

export const handlePhantomOrderPrices = wrap(async (event: SubstrateEvent): Promise<void> => {
	const blockNumber = event.block.block.header.number.toBigInt()
	if (blockNumber % 10n !== 0n) return
	const blockHash = event.block.block.header.hash.toString()

	// Use the storage value to identify the current interval — only its commitment is needed.
	const activeCommitment = await getActiveCommitment(blockHash)
	if (!activeCommitment) return

	// The anchor order tells us createdAtBlock, which all pairs in the same interval share.
	const anchor = await PhantomOrder.get(activeCommitment)
	if (!anchor) return

	// Fetch every pair registered in the same interval.
	const phantomOrders = await PhantomOrder.getByCreatedAtBlock(anchor.createdAtBlock, { limit: 100 })
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
				const callData = decoded.callData
				const solver = decoded.sender

				const fillData = extractFillData(callData, gatewayAddress)
				if (!fillData?.outputs.length) continue

				const simOk = await simulateBid(evmUrl, solver, callData, gatewayAddress, phantom.id)
				if (!simOk) continue

				const output = fillData.outputs[0]
				const tokenAddress = bytes32ToBytes20(output.token)
				const totalBalance = await getTotalSolverBalance(evmUrl, phantom.chain, tokenAddress, solver)

				prices.push(output.amount)
				if (totalBalance > bestLpBalance) bestLpBalance = totalBalance
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
