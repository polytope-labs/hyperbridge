import { SubstrateEvent } from "@subql/types"
import { decodeFunctionData, decodeAbiParameters } from "viem"
import { hexToU8a } from "@polkadot/util"
import { Struct, Bytes, Vector, u8 } from "scale-ts"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/intent-gateway-v2-addresses"
import { PhantomOrderBid, PhantomOrderBidOutput } from "@/configs/src/types"

// ─── ERC-7821 batch-execute ABI ──────────────────────────────────────────────

const ERC7821_ABI = [
	{
		name: "execute",
		type: "function",
		inputs: [
			{ name: "mode", type: "bytes32" },
			{ name: "executionData", type: "bytes" },
		],
		outputs: [],
	},
] as const

const CALL_COMPONENTS = [
	{ name: "target", type: "address" },
	{ name: "value", type: "uint256" },
	{ name: "data", type: "bytes" },
] as const

// ─── IntentGatewayV2 fillOrder ABI (minimal — only what we decode) ────────────

const FILL_ORDER_ABI = [
	{
		name: "fillOrder",
		type: "function",
		inputs: [
			{
				type: "tuple",
				name: "order",
				components: [
					{ type: "bytes32", name: "user" },
					{ type: "bytes", name: "source" },
					{ type: "bytes", name: "destination" },
					{ type: "uint256", name: "deadline" },
					{ type: "uint256", name: "nonce" },
					{ type: "uint256", name: "fees" },
					{ type: "address", name: "session" },
					{
						type: "tuple",
						name: "predispatch",
						components: [
							{
								type: "tuple[]",
								name: "assets",
								components: [
									{ type: "bytes32", name: "token" },
									{ type: "uint256", name: "amount" },
								],
							},
							{ type: "bytes", name: "call" },
						],
					},
					{
						type: "tuple[]",
						name: "inputs",
						components: [
							{ type: "bytes32", name: "token" },
							{ type: "uint256", name: "amount" },
						],
					},
					{
						type: "tuple",
						name: "output",
						components: [
							{ type: "bytes32", name: "beneficiary" },
							{
								type: "tuple[]",
								name: "assets",
								components: [
									{ type: "bytes32", name: "token" },
									{ type: "uint256", name: "amount" },
								],
							},
							{ type: "bytes", name: "call" },
						],
					},
				],
			},
			{
				type: "tuple",
				name: "options",
				components: [
					{ type: "uint256", name: "relayerFee" },
					{ type: "uint256", name: "nativeDispatchFee" },
					{
						type: "tuple[]",
						name: "outputs",
						components: [
							{ type: "bytes32", name: "token" },
							{ type: "uint256", name: "amount" },
						],
					},
				],
			},
		],
		outputs: [],
	},
] as const

// ─── SCALE codec for PackedUserOperation (mirrors SDK's definition) ──────────

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

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

// Block override value that makes phantom orders (deadline = 0) appear unexpired.
// With block.number = 0, the gateway's expiry check (block.number > deadline)
// evaluates to 0 > 0 = false, so the fill proceeds against real on-chain state.
const BLOCK_OVERRIDE_ZERO = "0x0"

// ─── Types ───────────────────────────────────────────────────────────────────

interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

interface TokenAmount {
	token: string
	amount: bigint
}

interface FillData {
	outputs: TokenAmount[]
	beneficiary: string
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

export function bytesToHex(bytes: Uint8Array | number[]): `0x${string}` {
	return `0x${Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("")}` as `0x${string}`
}

/** Strips the left-zero padding from a bytes32 address value. */
export function bytes32ToAddress(bytes32: string): string {
	return `0x${bytes32.replace("0x", "").slice(24)}`
}

/**
 * Decodes the ERC-7821 execute batch calldata and extracts all inner calls.
 * Returns null when the calldata does not match the execute function.
 */
export function decodeERC7821Execute(callData: `0x${string}`): Array<{ target: string; value: bigint; data: `0x${string}` }> | null {
	try {
		const decoded = decodeFunctionData({ abi: ERC7821_ABI, data: callData })
		if (decoded.functionName !== "execute" || !decoded.args || decoded.args.length < 2) return null

		const executionData = decoded.args[1] as `0x${string}`
		const [calls] = decodeAbiParameters([{ type: "tuple[]", components: CALL_COMPONENTS }], executionData) as any

		return calls.map((c: any) => ({ target: c.target as string, value: c.value as bigint, data: c.data as `0x${string}` }))
	} catch {
		return null
	}
}

/**
 * Searches the ERC-7821 batch for a `fillOrder` call to IntentGatewayV2 and
 * returns the proposed output amounts and the fill beneficiary.
 */
export function extractFillData(callData: `0x${string}`, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821Execute(callData)
	if (!calls) return null

	const normalizedGateway = gatewayAddress.toLowerCase()

	for (const call of calls) {
		if (call.target.toLowerCase() !== normalizedGateway) continue
		try {
			const decoded = decodeFunctionData({ abi: FILL_ORDER_ABI, data: call.data })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue

			const order = decoded.args[0] as { output: { beneficiary: string } }
			const options = decoded.args[1] as { outputs: { token: string; amount: bigint }[] }
			if (!options.outputs || options.outputs.length === 0) continue

			return {
				outputs: options.outputs.map((o) => ({ token: o.token as string, amount: o.amount })),
				beneficiary: bytes32ToAddress(order.output.beneficiary),
			}
		} catch {
			continue
		}
	}

	return null
}

/**
 * Fetches all current bids for an order commitment via the custom
 * `intents_getBidsForOrder` substrate RPC method.
 */
async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
	const response = await fetchWithRetry(nodeUrl, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "intents_getBidsForOrder", params: [commitment] }),
	})
	const data = await response.json()
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

/**
 * Reads the active phantom order commitment and chain from `CurrentPhantomOrder`
 * storage at the given block hash. Returns null when the storage is empty or
 * decoding fails.
 *
 * Storage layout (SCALE): `H256 (32 bytes) | u32 LE (4 bytes) | compact Vec<u8>`
 */
async function getActivePhantomCommitment(blockHash: string): Promise<{ commitment: string; chain: string } | null> {
	try {
		const storageKey = api.query.intentsCoprocessor.currentPhantomOrder.key()
		const rawResult = await api.rpc.state.getStorage(storageKey, blockHash)

		const hex: string = rawResult.toHex()
		if (!hex || hex === "0x") return null

		const bytes = Buffer.from(hex.replace("0x", ""), "hex")
		if (bytes.length < 37) return null

		const commitment = "0x" + bytes.slice(0, 32).toString("hex")

		// bytes[32..36] = created_at_block u32 LE (not needed here)
		const compactByte = bytes[36]
		const mode = compactByte & 0x03
		let chainStart: number
		let chainLen: number

		if (mode === 0) {
			chainLen = compactByte >> 2
			chainStart = 37
		} else if (mode === 1) {
			chainLen = (compactByte | (bytes[37] << 8)) >>> 2
			chainStart = 38
		} else {
			return null
		}

		const chain = bytes.slice(chainStart, chainStart + chainLen).toString("utf8")
		return { commitment, chain }
	} catch {
		return null
	}
}

/**
 * Simulates the filler's ERC-7821 execute batch against real on-chain state
 * via `eth_call`, with a block number override of 0. The override makes the
 * gateway's expiry check (block.number > deadline) pass for phantom orders
 * whose deadline is 0. Solver balances and allowances are NOT overridden —
 * the simulation reflects the solver's actual ability to back its quote.
 *
 * Returns `true` when the call succeeds, `false` when it reverts, and `null`
 * when the result cannot be interpreted (RPC error, unsupported node).
 */
async function simulateBid(evmRpcUrl: string, solver: string, callData: `0x${string}`): Promise<boolean | null> {
	try {
		const response = await fetchWithRetry(evmRpcUrl, {
			method: "POST",
			headers: { accept: "application/json", "content-type": "application/json" },
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_call",
				params: [
					{ to: solver, data: callData },
					"latest",
					{},
					{ number: BLOCK_OVERRIDE_ZERO },
				],
			}),
		})

		const result = await response.json()
		if (result.error) return null

		return true
	} catch {
		return null
	}
}

// ─── Handler ─────────────────────────────────────────────────────────────────

export const handlePhantomBidPlaced = wrap(async (event: SubstrateEvent): Promise<void> => {
	logger.info("Saw IntentsCoprocessor.BidPlaced")

	const [fillerData, commitmentData, depositData] = event.event.data

	const commitment = commitmentData.toString()
	const deposit = BigInt(depositData.toString())

	const host = getHostStateMachine(chainId)
	const blockHash = event.block.block.header.hash.toString()
	const blockNumber = event.block.block.header.number.toBigInt()

	const activePhantom = await getActivePhantomCommitment(blockHash)
	if (!activePhantom || activePhantom.commitment.toLowerCase() !== commitment.toLowerCase()) {
		// Regular order bid — not a phantom; nothing to index here.
		return
	}

	const chain = activePhantom.chain
	const blockTimestamp = await getBlockTimestamp(blockHash, host)

	const nodeUrl = replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "")
	if (!nodeUrl) {
		logger.warn(`No RPC URL for host ${host} — cannot fetch bid details`)
		return
	}

	let bids: RpcBidInfo[] = []
	try {
		bids = await fetchBidsForOrder(nodeUrl, commitment)
	} catch (err) {
		logger.warn({ err, commitment }, "intents_getBidsForOrder RPC failed")
	}

	const fillerHex = fillerData.toHex()
	const rpcBid = bids.find((b) => b.filler.toLowerCase() === fillerHex.toLowerCase())

	const bidId = `${commitment}-${fillerHex}`

	const existing = await PhantomOrderBid.get(bidId)
	if (existing) {
		logger.warn(`PhantomOrderBid ${bidId} already indexed — skipping`)
		return
	}

	let fillData: FillData | null = null
	let simulationSuccess: boolean | null = null

	if (rpcBid?.user_op) {
		try {
			const decoded = PackedUserOperationCodec.dec(hexToU8a(rpcBid.user_op))
			const callData = bytesToHex(decoded.callData)
			const solver = bytesToHex(decoded.sender)

			const gatewayAddress = INTENT_GATEWAY_V2_ADDRESSES[chain as keyof typeof INTENT_GATEWAY_V2_ADDRESSES]
			if (gatewayAddress) {
				fillData = extractFillData(callData, gatewayAddress)
			}

			const evmUrl = replaceWebsocketWithHttp(ENV_CONFIG[chain] ?? "")
			if (evmUrl) {
				simulationSuccess = await simulateBid(evmUrl, solver, callData)
			}
		} catch (err) {
			logger.warn({ err, bidId }, "Failed to decode UserOp for phantom bid")
		}
	}

	const bid = PhantomOrderBid.create({
		id: bidId,
		commitment,
		filler: fillerHex,
		deposit,
		blockNumber,
		blockTimestamp: timestampToDate(blockTimestamp),
		createdAt: timestampToDate(blockTimestamp),
		simulationSuccess: simulationSuccess ?? undefined,
	})
	await bid.save()

	if (fillData) {
		for (const [i, output] of fillData.outputs.entries()) {
			const bidOutput = PhantomOrderBidOutput.create({
				id: `${bidId}-${i}`,
				bidId,
				token: output.token,
				amount: output.amount,
			})
			await bidOutput.save()
		}
	}

	logger.info(
		{
			bidId,
			chain,
			outputCount: fillData?.outputs.length ?? 0,
			simulationSuccess,
		},
		"PhantomOrderBid indexed",
	)
})
