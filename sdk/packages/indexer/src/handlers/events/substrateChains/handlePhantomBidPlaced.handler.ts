import { SubstrateEvent } from "@subql/types"
import { decodeFunctionData, decodeAbiParameters, encodeAbiParameters, encodeFunctionData, keccak256 } from "viem"
import { hexToU8a } from "@polkadot/util"
import { Struct, Bytes, Vector, u8 } from "scale-ts"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { fetchWithRetry } from "@/utils/fetch-retry.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/intent-gateway-v2-addresses"
import { PhantomOrder, PhantomOrderBid, PhantomOrderBidOutput } from "@/configs/src/types"

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

// ─── ERC-20 transferFrom ABI (used in simulation) ────────────────────────────

const TRANSFER_FROM_ABI = [
	{
		name: "transferFrom",
		type: "function",
		inputs: [
			{ name: "from", type: "address" },
			{ name: "to", type: "address" },
			{ name: "amount", type: "uint256" },
		],
		outputs: [{ type: "bool" }],
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
const MAX_UINT256 = `0x${"f".repeat(64)}` as `0x${string}`

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
 * Computes the OZ ERC-20 storage slot for `balances[account]` (mapping at slot 0).
 */
export function balanceOfSlot(account: string): `0x${string}` {
	return keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [account as `0x${string}`, 0n]))
}

/**
 * Computes the OZ ERC-20 storage slot for `allowances[owner][spender]` (mapping at slot 1).
 */
export function allowanceSlot(owner: string, spender: string): `0x${string}` {
	const ownerHash = keccak256(
		encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [owner as `0x${string}`, 1n]),
	)
	return keccak256(
		encodeAbiParameters([{ type: "address" }, { type: "bytes32" }], [spender as `0x${string}`, ownerHash]),
	)
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
 * Simulates each ERC-20 `transferFrom(solver → beneficiary, amount)` via
 * `eth_call` using state overrides that grant the solver infinite balance and
 * allowance on the token contract (OZ ERC-20 storage layout: slot 0 = balances,
 * slot 1 = allowances).
 *
 * Returns `true` when every simulated transfer succeeds, `false` when at least
 * one reverts, and `null` when the simulation could not be run (e.g. RPC error,
 * unsupported node).
 */
async function simulateTokenTransfers(
	evmRpcUrl: string,
	gatewayAddress: string,
	solver: string,
	beneficiary: string,
	outputs: TokenAmount[],
): Promise<boolean | null> {
	try {
		for (const output of outputs) {
			const tokenAddr = bytes32ToAddress(output.token)
			if (tokenAddr.toLowerCase() === ZERO_ADDRESS) continue

			// Build state overrides: give solver infinite balance and allowance.
			const stateOverride = {
				[tokenAddr]: {
					stateDiff: {
						[balanceOfSlot(solver)]: MAX_UINT256,
						[allowanceSlot(solver, gatewayAddress)]: MAX_UINT256,
					},
				},
			}

			const data = encodeFunctionData({
				abi: TRANSFER_FROM_ABI,
				functionName: "transferFrom",
				args: [solver as `0x${string}`, beneficiary as `0x${string}`, output.amount],
			})

			const response = await fetchWithRetry(evmRpcUrl, {
				method: "POST",
				headers: { accept: "application/json", "content-type": "application/json" },
				body: JSON.stringify({
					id: 1,
					jsonrpc: "2.0",
					method: "eth_call",
					params: [{ from: gatewayAddress, to: tokenAddr, data }, "latest", stateOverride],
				}),
			})

			const result = await response.json()

			if (result.error) return null

			// Decode the bool return value; anything that isn't a clean `true` is a failure.
			const returnedFalse =
				!result.result ||
				result.result === "0x" ||
				result.result === "0x0000000000000000000000000000000000000000000000000000000000000000"

			if (returnedFalse) return false
		}

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

	const phantomOrder = await PhantomOrder.get(commitment)
	if (!phantomOrder) {
		// Regular order bid — not a phantom; nothing to index here.
		return
	}

	const host = getHostStateMachine(chainId)
	const blockHash = event.block.block.header.hash.toString()
	const blockNumber = event.block.block.header.number.toBigInt()
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

			const gatewayAddress = INTENT_GATEWAY_V2_ADDRESSES[phantomOrder.chain as keyof typeof INTENT_GATEWAY_V2_ADDRESSES]
			if (gatewayAddress) {
				fillData = extractFillData(callData, gatewayAddress)

				const evmUrl = replaceWebsocketWithHttp(ENV_CONFIG[phantomOrder.chain] ?? "")
				if (evmUrl && fillData && solver) {
					simulationSuccess = await simulateTokenTransfers(
						evmUrl,
						gatewayAddress,
						solver,
						fillData.beneficiary,
						fillData.outputs,
					)
				}
			}
		} catch (err) {
			logger.warn({ err, bidId }, "Failed to decode UserOp for phantom bid")
		}
	}

	const bid = PhantomOrderBid.create({
		id: bidId,
		orderId: commitment,
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
			chain: phantomOrder.chain,
			outputCount: fillData?.outputs.length ?? 0,
			simulationSuccess,
		},
		"PhantomOrderBid indexed",
	)
})
