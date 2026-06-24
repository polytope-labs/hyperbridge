import { SubstrateEvent } from "@subql/types"
import { decodeFunctionData, decodeAbiParameters, keccak256, concat, toHex } from "viem"
import { hexToU8a, u8aToHex } from "@polkadot/util"
import { Bytes, Struct, u8, Vector } from "scale-ts"
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

// ─── Inlined SDK helpers (avoids bundling TronWeb which crashes in SubQuery VM2) ─

type HexString = `0x${string}`

const PackedUserOperationCodec = Struct({
	sender: Bytes(20),
	nonce: Bytes(32),
	initCode: Vector(u8),
	callData: Vector(u8),
	accountGasLimits: Bytes(32),
	preVerificationGas: Bytes(32),
	gasFees: Bytes(32),
	paymasterAndData: Vector(u8),
	signature: Vector(u8),
})

function decodeUserOpScale(hex: string): { sender: string; callData: string } {
	const d = PackedUserOperationCodec.dec(hexToU8a(hex))
	return {
		sender: u8aToHex(new Uint8Array(d.sender)),
		callData: u8aToHex(new Uint8Array(d.callData)),
	}
}

const ERC7821_ABI = [
	{ name: "execute", type: "function", inputs: [{ name: "mode", type: "bytes32" }, { name: "executionData", type: "bytes" }], outputs: [] },
] as const

function decodeERC7821ExecuteBatch(callData: string): Array<{ target: string; value: bigint; data: string }> | null {
	try {
		const decoded = decodeFunctionData({ abi: ERC7821_ABI, data: callData as HexString })
		if (decoded.functionName !== "execute" || !decoded.args || decoded.args.length < 2) return null
		const executionData = decoded.args[1] as HexString
		const [calls] = decodeAbiParameters(
			[{ type: "tuple[]", components: [{ name: "target", type: "address" }, { name: "value", type: "uint256" }, { name: "data", type: "bytes" }] }],
			executionData,
		) as [Array<{ target: string; value: bigint; data: string }>]
		return calls.map((c) => ({ target: c.target, value: c.value, data: c.data }))
	} catch {
		return null
	}
}

// fillOrder(Order order, FillOptions options) — only the outputs field from FillOptions is needed
const FILL_ORDER_ABI = [
	{
		name: "fillOrder",
		type: "function",
		inputs: [
			{
				name: "order",
				type: "tuple",
				components: [
					{ name: "user", type: "bytes32" },
					{ name: "source", type: "bytes" },
					{ name: "destination", type: "bytes" },
					{ name: "deadline", type: "uint256" },
					{ name: "nonce", type: "uint256" },
					{ name: "fees", type: "uint256" },
					{ name: "session", type: "address" },
					{
						name: "predispatch",
						type: "tuple",
						components: [
							{ name: "assets", type: "tuple[]", components: [{ name: "token", type: "bytes32" }, { name: "amount", type: "uint256" }] },
							{ name: "call", type: "bytes" },
						],
					},
					{ name: "inputs", type: "tuple[]", components: [{ name: "token", type: "bytes32" }, { name: "amount", type: "uint256" }] },
					{
						name: "output",
						type: "tuple",
						components: [
							{ name: "beneficiary", type: "bytes32" },
							{ name: "assets", type: "tuple[]", components: [{ name: "token", type: "bytes32" }, { name: "amount", type: "uint256" }] },
							{ name: "call", type: "bytes" },
						],
					},
				],
			},
			{
				name: "options",
				type: "tuple",
				components: [
					{ name: "outputs", type: "tuple[]", components: [{ name: "token", type: "bytes32" }, { name: "amount", type: "uint256" }] },
					{ name: "nativeDispatchFee", type: "uint256" },
					{ name: "proverData", type: "bytes" },
				],
			},
		],
		outputs: [],
	},
] as const

// Computes the EVM storage slot for _orders[commitment][inputToken] in IntentGatewayV2.
// _orders is a mapping(bytes32 => mapping(address => uint256)) at storage slot 10.
// inputTokenBytes32 is the bytes32 token field from the order (address left-padded to 32 bytes),
// which is identical to the abi.encode(address) key Solidity uses for the inner mapping.
function ordersStorageSlot(commitment: HexString, inputTokenBytes32: HexString): HexString {
	const ORDERS_SLOT = BigInt(10)
	const innerSlot = keccak256(concat([commitment, toHex(ORDERS_SLOT, { size: 32 })]))
	return keccak256(concat([inputTokenBytes32, innerSlot]))
}

// ─── Types ───────────────────────────────────────────────────────────────────

interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

interface FillData {
	outputs: { token: string; amount: bigint }[]
	fillCalldata: HexString
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			const decoded = decodeFunctionData({ abi: FILL_ORDER_ABI, data: call.data as HexString })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const options = decoded.args[1] as unknown as { outputs: { token: string; amount: bigint }[] }
			if (!options.outputs?.length) continue
			return {
				outputs: options.outputs.map((o) => ({ token: o.token as string, amount: o.amount })),
				fillCalldata: call.data as HexString,
			}
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

// Simulates a fillOrder call from the solver on the IntentGatewayV2.
// Block number is overridden to 0 so the deadline check (deadline < block.number)
// always passes for any non-zero deadline. The _orders[commitment][inputToken]
// storage slot is injected via state diff so the gateway sees the phantom escrow
// and releases it to the solver on success.
async function simulateBid(
	evmRpcUrl: string,
	solver: string,
	fillCalldata: HexString,
	gatewayAddress: string,
	commitment: string,
	inputTokenBytes32: HexString,
	inputAmount: bigint,
): Promise<boolean> {
	try {
		const storageSlot = ordersStorageSlot(commitment as HexString, inputTokenBytes32)
		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [
					{ from: solver, to: gatewayAddress, data: fillCalldata },
					"latest",
					{
						[gatewayAddress]: {
							stateDiff: {
								[storageSlot]: toHex(inputAmount, { size: 32 }),
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

// Sums the solver's balance of the output token across ALL configured EVM chains.
// Uses the same token address on every chain
// Returns 0 for chains where the token address does not exist.
async function getTotalSolverBalanceAllChains(outputTokenAddress: string, solver: string): Promise<bigint> {
	const checks = Object.entries(ENV_CONFIG)
		.filter(([chain]) => chain.startsWith("EVM-"))
		.map(async ([chain, url]) => {
			const evmRpcUrl = replaceWebsocketWithHttp(url ?? "")
			if (!evmRpcUrl) return 0n
			return getTotalSolverBalance(evmRpcUrl, chain, outputTokenAddress, solver)
		})
	const balances = await Promise.all(checks)
	return balances.reduce((acc, b) => acc + b, 0n)
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
				const decoded = decodeUserOpScale(bid.user_op)
				const callData = decoded.callData as HexString
				const solver = decoded.sender

				const fillData = extractFillData(callData, gatewayAddress)
				if (!fillData?.outputs.length) continue

				const simOk = await simulateBid(
					evmUrl,
					solver,
					fillData.fillCalldata,
					gatewayAddress,
					phantom.id,
					phantom.tokenA as HexString,
					BigInt(phantom.standardAmount.toString()),
				)
				if (!simOk) continue

				const output = fillData.outputs[0]
				const outputTokenAddress = bytes32ToBytes20(output.token)
				const totalBalance = await getTotalSolverBalanceAllChains(outputTokenAddress, solver)

				prices.push(output.amount)
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
