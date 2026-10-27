import { ENTRY_POINT_V08, ENTRY_POINT_V09 } from "@/configs/chain"
import { BidImpl } from "@/protocols/intents/Bid"
import { CryptoUtils, LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import { encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { encodeFillOrder } from "@/protocols/intents/fillOrderCodec"
import { orderCommitment, transformOrderForContract } from "@/protocols/intents/utils"
import type { ERC7821Call, FillOptions, HexString, Order, PackedUserOperation } from "@/types"
import {
	concat,
	encodeAbiParameters,
	hashTypedData,
	pad,
	recoverAddress,
	slice,
	toEventSelector,
	type Log,
	type TypedDataDomain,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { describe, expect, it, vi } from "vitest"

const CHAIN_ID = 8453
const TOKEN = pad("0x55", { size: 32 }) as HexString
const solver = privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
const SESSION_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const session = privateKeyToAccount(SESSION_KEY)
const UNKNOWN_ENTRY_POINT = "0x0000000000000000000000000000000000004337" as HexString

const BY_USER_OP_HASH = {
	address: "0x0000000000000000000000000000000000000a01" as HexString,
	typehash: SELECT_SOLVER_TYPEHASH,
}
const BY_ADDRESS = {
	address: "0x0000000000000000000000000000000000000a02" as HexString,
	typehash: LEGACY_SELECT_SOLVER_TYPEHASH,
}
type Gateway = typeof BY_USER_OP_HASH

const placed: Order = {
	user: TOKEN,
	source: "EVM-1",
	destination: `EVM-${CHAIN_ID}`,
	deadline: 100n,
	nonce: 1n,
	fees: 0n,
	session: session.address,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: TOKEN, amount: 100n }],
	output: { beneficiary: TOKEN, assets: [{ token: TOKEN, amount: 100n }], call: "0x" },
}
const COMMITMENT = orderCommitment(placed)
const order: Order = { ...placed, id: COMMITMENT }

const fillOptions: FillOptions = {
	outputs: [{ token: TOKEN, amount: 100n }],
	inputs: [{ token: TOKEN, amount: 100n }],
	relayerFee: 0n,
	nativeDispatchFee: 0n,
	validUntil: 0n,
}

/** A call of `fillOrder` for `filled` on `gateway`. */
function fillCall(gateway: Gateway, filled: Order = placed): ERC7821Call {
	return {
		target: gateway.address,
		value: 0n,
		data: encodeFillOrder(transformOrderForContract(filled) as Order, fillOptions),
	}
}

/** A batch whose only call fills `order` on `gateway`. */
function fillCallData(gateway: Gateway): HexString {
	return encodeERC7821ExecuteBatch([fillCall(gateway)])
}

/**
 * A bid as `BidManager.prepareSubmitBid` builds it for `order`: `callData`, a nonce keyed to the
 * order, its session and the calldata, and a signature of the commitment, then the solver's
 * signature for `entryPoint`.
 */
async function signedFor(
	entryPoint: HexString,
	{
		callData = fillCallData(BY_USER_OP_HASH),
		nonce = CryptoUtils.bidNonceKey(COMMITMENT, session.address, callData) << 64n,
	}: { callData?: HexString; nonce?: bigint } = {},
): Promise<PackedUserOperation> {
	const userOp: PackedUserOperation = {
		sender: solver.address,
		nonce,
		initCode: "0x",
		callData,
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x",
	}
	const solverSignature = await solver.signTypedData(
		CryptoUtils.packedUserOpTypedData(userOp, entryPoint, BigInt(CHAIN_ID)),
	)
	return { ...userOp, signature: concat([COMMITMENT, solverSignature]) }
}

/** The bundle the bundler lands: only this operation, emitted by the EntryPoint it was sent to. */
function bundle(entryPoint: HexString, userOpHash: HexString): Log[] {
	const operationEvent = toEventSelector("UserOperationEvent(bytes32,address,address,uint256,bool,uint256,uint256)")
	const data = encodeAbiParameters(
		[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
		[1n, true, 0n, 0n],
	)
	return [
		{ address: entryPoint, topics: [toEventSelector("BeforeExecution()")], data: "0x" },
		{
			address: entryPoint,
			topics: [operationEvent, userOpHash, pad(solver.address, { size: 32 }), pad("0x", { size: 32 })],
			data,
		},
	] as unknown as Log[]
}

function bidOn(gateway: Gateway, userOp: PackedUserOperation) {
	const sent = { entryPoint: "0x" as HexString, userOpHash: "0x" as HexString }
	const client = {
		chain: { id: CHAIN_ID },
		readContract: vi.fn(async ({ functionName }: { functionName: string }) => {
			if (functionName !== "SELECT_SOLVER_TYPEHASH") throw new Error(`unexpected read ${functionName}`)
			return gateway.typehash
		}),
		getStorageAt: vi.fn(async () => pad("0x01", { size: 32 })),
		waitForTransactionReceipt: async () => ({ logs: bundle(sent.entryPoint, sent.userOpHash) }),
	}
	const ctx = {
		bundlerUrl: "https://bundler.invalid",
		dest: {
			config: { stateMachineId: `EVM-${CHAIN_ID}` },
			configService: { getIntentGatewayAddress: () => gateway.address },
			client,
		},
	} as never
	const crypto = new CryptoUtils(ctx)
	const sendBundler = vi.spyOn(crypto, "sendBundler").mockImplementation(async (method, params) => {
		if (method !== "eth_sendUserOperation") return { receipt: { transactionHash: `0x${"77".repeat(32)}` } }
		const [, entryPoint] = params as [unknown, HexString]
		sent.entryPoint = entryPoint
		sent.userOpHash = CryptoUtils.computeUserOpHash(userOp, entryPoint, BigInt(CHAIN_ID))
		return sent.userOpHash as never
	})
	const bid = new BidImpl({
		ctx,
		crypto,
		order,
		fillerBid: { filler: "solver", bid: CryptoUtils.bidId(userOp.callData), userOp, deposit: 0n },
		fillOptions,
		priceOutputs: async () => null,
		sessionPrivateKey: SESSION_KEY,
	})
	return { bid, client, sendBundler }
}

/** The params of the first bundler call to `method`. */
function bundlerParams(sendBundler: ReturnType<typeof bidOn>["sendBundler"], method: string): unknown[] {
	const call = sendBundler.mock.calls.find(([sent]) => sent === method)
	if (!call) throw new Error(`no ${method} sent`)
	return call[1] as unknown[]
}

/** The session's selection signature, the last 65 bytes of the signature sent to the bundler. */
function sentSelection(sendBundler: ReturnType<typeof bidOn>["sendBundler"]): HexString {
	const [sentOp] = bundlerParams(sendBundler, "eth_sendUserOperation") as [{ signature: HexString }]
	return slice(sentOp.signature, 97)
}

function gatewayDomain(gateway: Gateway): TypedDataDomain {
	return { name: "IntentGateway", version: "2", chainId: CHAIN_ID, verifyingContract: gateway.address }
}

/** The digest a session signs to select the v0.9 bid `userOp` on `gateway`. */
function userOpHashSelectionDigest(gateway: Gateway, userOp: PackedUserOperation): HexString {
	return hashTypedData({
		domain: gatewayDomain(gateway),
		types: {
			SelectSolver: [
				{ name: "commitment", type: "bytes32" },
				{ name: "userOpHash", type: "bytes32" },
			],
		},
		primaryType: "SelectSolver",
		message: {
			commitment: COMMITMENT,
			userOpHash: CryptoUtils.computeUserOpHash(userOp, ENTRY_POINT_V09, BigInt(CHAIN_ID)),
		},
	})
}

describe("BidImpl on EntryPoint v0.9", () => {
	it("simulates and executes a bid signed for EntryPoint v0.9 there, selected by its userOpHash", async () => {
		const userOp = await signedFor(ENTRY_POINT_V09)
		const { bid, sendBundler } = bidOn(BY_USER_OP_HASH, userOp)

		await bid.simulate()
		await bid.execute()

		expect(bundlerParams(sendBundler, "eth_estimateUserOperationGas")[1]).toBe(ENTRY_POINT_V09)
		expect(bundlerParams(sendBundler, "eth_sendUserOperation")[1]).toBe(ENTRY_POINT_V09)

		const digest = userOpHashSelectionDigest(BY_USER_OP_HASH, userOp)
		expect(await recoverAddress({ hash: digest, signature: sentSelection(sendBundler) })).toBe(session.address)
	})

	it.each([
		["EntryPoint v0.8", ENTRY_POINT_V08],
		["an unknown EntryPoint", UNKNOWN_ENTRY_POINT],
	])("drops a bid signed for %s without asking the bundler", async (_name, entryPoint) => {
		const { bid, sendBundler } = bidOn(BY_USER_OP_HASH, await signedFor(entryPoint))

		await expect(bid.simulate()).rejects.toThrow("is not signed for EntryPoint v0.9")
		await expect(bid.execute()).rejects.toThrow("is not signed for EntryPoint v0.9")
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("drops a bid whose signature opens with another order's id, without asking the bundler", async () => {
		const userOp = await signedFor(ENTRY_POINT_V09)
		// The solver's own signature still recovers: the prefix is not part of the userOpHash.
		const otherOrder = { ...userOp, signature: concat([`0x${"77".repeat(32)}`, slice(userOp.signature, 32)]) }
		const { bid, client, sendBundler } = bidOn(BY_USER_OP_HASH, otherOrder)

		await expect(bid.simulate()).rejects.toThrow(`is not signed for order ${COMMITMENT}`)
		await expect(bid.execute()).rejects.toThrow(`is not signed for order ${COMMITMENT}`)
		expect(client.readContract).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("drops a bid whose nonce key does not bind the order, without asking the bundler", async () => {
		const callData = fillCallData(BY_USER_OP_HASH)
		const nonce = CryptoUtils.bidNonceKey(COMMITMENT, solver.address, callData) << 64n
		const { bid, client, sendBundler } = bidOn(
			BY_USER_OP_HASH,
			await signedFor(ENTRY_POINT_V09, { callData, nonce }),
		)

		await expect(bid.simulate()).rejects.toThrow("has a nonce key that does not bind order")
		await expect(bid.execute()).rejects.toThrow("has a nonce key that does not bind order")
		expect(client.readContract).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("accepts a later sequence of the bid's nonce key", async () => {
		// The same calldata run once already, as when a partially filled order is bid on again.
		const callData = fillCallData(BY_USER_OP_HASH)
		const nonce = (CryptoUtils.bidNonceKey(COMMITMENT, session.address, callData) << 64n) | 1n
		const { bid, sendBundler } = bidOn(BY_USER_OP_HASH, await signedFor(ENTRY_POINT_V09, { callData, nonce }))

		await bid.simulate()

		expect(bundlerParams(sendBundler, "eth_estimateUserOperationGas")[1]).toBe(ENTRY_POINT_V09)
	})

	const another: Order = { ...placed, nonce: 2n }
	it.each([
		["another order", [fillCall(BY_USER_OP_HASH, another)]],
		["this order and another", [fillCall(BY_USER_OP_HASH), fillCall(BY_USER_OP_HASH, another)]],
		["this order on another gateway", [fillCall(BY_ADDRESS)]],
		["nothing", []],
	])("drops a bid whose calldata fills %s, without asking the bundler", async (_name, calls) => {
		const callData = encodeERC7821ExecuteBatch(calls)
		const { bid, client, sendBundler } = bidOn(BY_USER_OP_HASH, await signedFor(ENTRY_POINT_V09, { callData }))

		await expect(bid.simulate()).rejects.toThrow(`does not fill order ${COMMITMENT} alone`)
		await expect(bid.execute()).rejects.toThrow(`does not fill order ${COMMITMENT} alone`)
		expect(client.readContract).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("drops a bid on a gateway that selects by address", async () => {
		const { bid, sendBundler } = bidOn(
			BY_ADDRESS,
			await signedFor(ENTRY_POINT_V09, { callData: fillCallData(BY_ADDRESS) }),
		)

		await expect(bid.simulate()).rejects.toThrow("the destination gateway selects by address")
		await expect(bid.execute()).rejects.toThrow("the destination gateway selects by address")
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("reads a gateway's selection format once across bids", async () => {
		const gateway: Gateway = {
			address: "0x0000000000000000000000000000000000000a03",
			typehash: SELECT_SOLVER_TYPEHASH,
		}
		const callData = fillCallData(gateway)
		const first = bidOn(gateway, await signedFor(ENTRY_POINT_V09, { callData }))
		await first.bid.simulate()
		const second = bidOn(gateway, await signedFor(ENTRY_POINT_V09, { callData }))
		await second.bid.simulate()
		await second.bid.execute()

		expect(first.client.readContract).toHaveBeenCalledOnce()
		expect(second.client.readContract).not.toHaveBeenCalled()
	})
})
