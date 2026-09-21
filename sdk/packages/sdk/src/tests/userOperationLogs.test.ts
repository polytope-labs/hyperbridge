import { ABI } from "@/abis/IntentGatewayV2"
import { BidImpl, userOperationLogs } from "@/protocols/intents/Bid"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { HexString, Order, PackedUserOperation } from "@/types"
import { encodeAbiParameters, encodeEventTopics, pad, toEventSelector, type Log } from "viem"
import { afterEach, describe, expect, it, vi } from "vitest"

const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const GATEWAY = "0x6666666666666666666666666666666666666666" as HexString
const OURS = `0x${"aa".repeat(32)}` as HexString
const THEIRS = `0x${"bb".repeat(32)}` as HexString
const SOLVER_OTHER = "0x9999999999999999999999999999999999999999" as HexString

const BEFORE_EXECUTION = toEventSelector("BeforeExecution()")
const USER_OPERATION_EVENT = toEventSelector("UserOperationEvent(bytes32,address,address,uint256,bool,uint256,uint256)")

function log(address: HexString, topics: HexString[], data: HexString = "0x"): Log {
	return { address, topics, data } as unknown as Log
}

const beforeExecution = () => log(ENTRY_POINT, [BEFORE_EXECUTION])
const gatewayLog = (tag: string) => log(GATEWAY, [pad(`0x${tag}` as HexString, { size: 32 })])

function operationEvent(userOpHash: HexString, success = true, emitter = ENTRY_POINT): Log {
	const data = encodeAbiParameters(
		[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
		[1n, success, 0n, 0n],
	)
	return log(emitter, [USER_OPERATION_EVENT, userOpHash, pad(GATEWAY, { size: 32 }), pad("0x", { size: 32 })], data)
}

describe("userOperationLogs", () => {
	it("returns only the logs of the named operation inside a shared bundle", () => {
		const [theirFill, ourFill, laterFill] = [gatewayLog("01"), gatewayLog("02"), gatewayLog("03")]
		const logs = [
			beforeExecution(),
			theirFill,
			operationEvent(THEIRS),
			ourFill,
			operationEvent(OURS),
			laterFill,
			operationEvent(`0x${"cc".repeat(32)}`),
		]

		expect(userOperationLogs(logs, ENTRY_POINT, OURS)).toEqual([ourFill])
	})

	it("opens the first operation's range at BeforeExecution", () => {
		const ourFill = gatewayLog("02")
		const logs = [gatewayLog("00"), beforeExecution(), ourFill, operationEvent(OURS)]

		expect(userOperationLogs(logs, ENTRY_POINT, OURS)).toEqual([ourFill])
	})

	it("ignores an operation event that the EntryPoint did not emit", () => {
		const logs = [beforeExecution(), gatewayLog("02"), operationEvent(OURS, true, GATEWAY)]

		expect(() => userOperationLogs(logs, ENTRY_POINT, OURS)).toThrow("is not in the transaction")
	})

	it("rejects an operation that reverted", () => {
		const logs = [beforeExecution(), operationEvent(OURS, false)]

		expect(() => userOperationLogs(logs, ENTRY_POINT, OURS)).toThrow("reverted")
	})
})

describe("BidImpl.execute fill attribution", () => {
	const SOLVER = "0x2222222222222222222222222222222222222222" as HexString
	const TOKEN = pad("0x55", { size: 32 }) as HexString
	const COMMITMENT = `0x${"66".repeat(32)}` as HexString
	const userOp: PackedUserOperation = {
		sender: SOLVER,
		nonce: 1n,
		initCode: "0x",
		callData: "0x1234",
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x12",
	}
	const order: Order = {
		id: COMMITMENT,
		user: TOKEN,
		source: "EVM-1",
		destination: "EVM-8453",
		deadline: 100n,
		nonce: 1n,
		fees: 0n,
		session: SOLVER,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 100n }],
		output: { beneficiary: TOKEN, assets: [{ token: TOKEN, amount: 100n }], call: "0x" },
	}

	function partialFill(filler: HexString, amount: bigint, emitter = GATEWAY): Log {
		const legs = { type: "tuple[]", components: [{ type: "bytes32" }, { type: "uint256" }] } as const
		return log(
			emitter,
			encodeEventTopics({ abi: ABI, eventName: "PartialFill", args: { commitment: COMMITMENT } }) as HexString[],
			encodeAbiParameters([{ type: "address" }, legs, legs], [filler, [[TOKEN, amount]], [[TOKEN, amount]]]),
		)
	}

	afterEach(() => vi.unstubAllGlobals())

	it("credits only the fills its own operation made in the gateway", async () => {
		let userOpHash = "0x" as HexString
		// Another solver fills the same order earlier in the bundle, and a foreign contract imitates
		// the gateway's event inside our operation.
		const bundle = () => [
			beforeExecution(),
			partialFill(SOLVER_OTHER, 30n),
			operationEvent(THEIRS),
			partialFill(SOLVER, 10n),
			partialFill(SOLVER, 5n),
			partialFill(SOLVER, 50n, ENTRY_POINT),
			operationEvent(userOpHash),
		]
		const ctx = {
			bundlerUrl: "https://bundler.invalid",
			sessionKeyStorage: { getSessionKeyByAddress: async () => ({ privateKey: `0x${"01".repeat(32)}` }) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT, getIntentGatewayAddress: () => GATEWAY },
				client: {
					chain: { id: 8453 },
					waitForTransactionReceipt: async () => ({ logs: bundle() }),
				},
			},
		} as never
		const crypto = new CryptoUtils(ctx)
		const bid = new BidImpl({
			ctx,
			crypto,
			order,
			fillerBid: { filler: "solver", userOp, deposit: 0n },
			fillOptions: {
				outputs: [{ token: TOKEN, amount: 100n }],
				inputs: [{ token: TOKEN, amount: 100n }],
				relayerFee: 0n,
				nativeDispatchFee: 0n,
				validUntil: 0n,
			},
			priceOutputs: async () => null,
		})
		vi.spyOn(crypto, "sendBundler").mockImplementation(async (method, params) => {
			if (method !== "eth_sendUserOperation") return { receipt: { transactionHash: `0x${"77".repeat(32)}` } }
			const sent = (params as [Record<string, HexString>])[0]
			userOpHash = CryptoUtils.computeUserOpHash({ ...userOp, signature: sent.signature }, ENTRY_POINT, 8453n)
			return userOpHash as never
		})

		expect(await bid.execute()).toMatchObject({
			fillStatus: "partial",
			filledAssets: [{ token: TOKEN, amount: 15n }],
		})
	})
})
