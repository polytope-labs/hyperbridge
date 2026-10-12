import { ENTRY_POINT_V08, ENTRY_POINT_V09 } from "@/configs/chain"
import { BidImpl } from "@/protocols/intents/Bid"
import { CryptoUtils, LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import type { HexString, Order, PackedUserOperation } from "@/types"
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
const COMMITMENT = `0x${"66".repeat(32)}` as HexString
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

const order: Order = {
	id: COMMITMENT,
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

/** A bid as `BidManager.prepareSubmitBid` builds it: the commitment, then the solver's signature for `entryPoint`. */
async function signedFor(entryPoint: HexString): Promise<PackedUserOperation> {
	const userOp: PackedUserOperation = {
		sender: solver.address,
		nonce: 1n,
		initCode: "0x",
		callData: "0x1234",
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
		call: vi.fn(async () => ({ data: "0x" })),
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
		fillOptions: {
			outputs: [{ token: TOKEN, amount: 100n }],
			inputs: [{ token: TOKEN, amount: 100n }],
			relayerFee: 0n,
			nativeDispatchFee: 0n,
			validUntil: 0n,
		},
		priceOutputs: async () => null,
		sessionPrivateKey: SESSION_KEY,
	})
	return { bid, client, sendBundler }
}

/** The session's selection signature, the last 65 bytes of the signature sent to the bundler. */
function sentSelection(sendBundler: ReturnType<typeof bidOn>["sendBundler"]): HexString {
	const [, [sentOp]] = sendBundler.mock.calls[0] as [string, [{ signature: HexString }]]
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

describe("BidImpl EntryPoint routing", () => {
	it("executes a bid signed for EntryPoint v0.9 there, selected by its userOpHash", async () => {
		const userOp = await signedFor(ENTRY_POINT_V09)
		const { bid, sendBundler } = bidOn(BY_USER_OP_HASH, userOp)

		await bid.simulate()
		await bid.execute()

		expect(sendBundler.mock.calls[0][0]).toBe("eth_sendUserOperation")
		expect((sendBundler.mock.calls[0][1] as unknown[])[1]).toBe(ENTRY_POINT_V09)

		const digest = userOpHashSelectionDigest(BY_USER_OP_HASH, userOp)
		expect(await recoverAddress({ hash: digest, signature: sentSelection(sendBundler) })).toBe(session.address)
	})

	it("executes a bid signed for EntryPoint v0.8 there, selected by its solver's address", async () => {
		const { bid, sendBundler } = bidOn(BY_ADDRESS, await signedFor(ENTRY_POINT_V08))

		await bid.simulate()
		await bid.execute()

		expect((sendBundler.mock.calls[0][1] as unknown[])[1]).toBe(ENTRY_POINT_V08)

		const digest = hashTypedData({
			domain: gatewayDomain(BY_ADDRESS),
			types: {
				SelectSolver: [
					{ name: "commitment", type: "bytes32" },
					{ name: "solver", type: "address" },
				],
			},
			primaryType: "SelectSolver",
			message: { commitment: COMMITMENT, solver: solver.address },
		})
		expect(await recoverAddress({ hash: digest, signature: sentSelection(sendBundler) })).toBe(session.address)
	})

	it("drops a bid signed for neither EntryPoint", async () => {
		const { bid, client, sendBundler } = bidOn(BY_USER_OP_HASH, await signedFor(UNKNOWN_ENTRY_POINT))

		await expect(bid.simulate()).rejects.toThrow("is not signed for a known EntryPoint")
		await expect(bid.execute()).rejects.toThrow("is not signed for a known EntryPoint")
		expect(client.call).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("drops a v0.8 bid on a gateway that selects by userOpHash", async () => {
		const { bid, client, sendBundler } = bidOn(BY_USER_OP_HASH, await signedFor(ENTRY_POINT_V08))

		await expect(bid.simulate()).rejects.toThrow("userOpHash selection does not pair with")
		await expect(bid.execute()).rejects.toThrow("userOpHash selection does not pair with")
		expect(client.call).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})

	it("drops a v0.9 bid on a gateway that selects by address", async () => {
		const { bid, client, sendBundler } = bidOn(BY_ADDRESS, await signedFor(ENTRY_POINT_V09))

		await expect(bid.simulate()).rejects.toThrow("address selection does not pair with")
		await expect(bid.execute()).rejects.toThrow("address selection does not pair with")
		expect(client.call).not.toHaveBeenCalled()
		expect(sendBundler).not.toHaveBeenCalled()
	})
})

describe("BidImpl selection format after a gateway upgrade", () => {
	it("reads the format again when a v0.9 bid meets a cached address selection", async () => {
		const gateway: Gateway = {
			address: "0x0000000000000000000000000000000000000a03",
			typehash: LEGACY_SELECT_SOLVER_TYPEHASH,
		}
		const before = bidOn(gateway, await signedFor(ENTRY_POINT_V08))
		await before.bid.simulate()
		expect(before.client.readContract).toHaveBeenCalledOnce()

		// The proxy is upgraded in place to an implementation that selects by userOpHash.
		gateway.typehash = SELECT_SOLVER_TYPEHASH
		const userOp = await signedFor(ENTRY_POINT_V09)
		const { bid, client, sendBundler } = bidOn(gateway, userOp)

		await bid.simulate()
		await bid.execute()

		expect(client.readContract).toHaveBeenCalledOnce()
		expect((sendBundler.mock.calls[0][1] as unknown[])[1]).toBe(ENTRY_POINT_V09)
		const digest = userOpHashSelectionDigest(gateway, userOp)
		expect(await recoverAddress({ hash: digest, signature: sentSelection(sendBundler) })).toBe(session.address)

		const stale = bidOn(gateway, await signedFor(ENTRY_POINT_V08))
		await expect(stale.bid.simulate()).rejects.toThrow("userOpHash selection does not pair with")
	})

	it("reads the format once for concurrent bids that miss the cached one", async () => {
		const gateway: Gateway = {
			address: "0x0000000000000000000000000000000000000a04",
			typehash: LEGACY_SELECT_SOLVER_TYPEHASH,
		}
		await bidOn(gateway, await signedFor(ENTRY_POINT_V08)).bid.simulate()

		gateway.typehash = SELECT_SOLVER_TYPEHASH
		const userOp = await signedFor(ENTRY_POINT_V09)
		const bids = [bidOn(gateway, userOp), bidOn(gateway, userOp), bidOn(gateway, userOp)]
		await Promise.all(bids.map(({ bid }) => bid.simulate()))

		const reads = bids.reduce((total, { client }) => total + client.readContract.mock.calls.length, 0)
		expect(reads).toBe(1)
	})

	it("does not read the format again when the bid pairs with the cached one", async () => {
		const gateway: Gateway = {
			address: "0x0000000000000000000000000000000000000a05",
			typehash: SELECT_SOLVER_TYPEHASH,
		}
		const first = bidOn(gateway, await signedFor(ENTRY_POINT_V09))
		await first.bid.simulate()
		const second = bidOn(gateway, await signedFor(ENTRY_POINT_V09))
		await second.bid.simulate()
		await second.bid.execute()

		expect(first.client.readContract).toHaveBeenCalledOnce()
		expect(second.client.readContract).not.toHaveBeenCalled()
	})
})
