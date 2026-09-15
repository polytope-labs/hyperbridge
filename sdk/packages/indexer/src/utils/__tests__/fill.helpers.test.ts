import { Interface } from "@ethersproject/abi"
import type { EthereumLog } from "@subql/types-ethereum"

import { IOrderV3OutputAsset } from "@/configs/src/types/models/IOrderV3OutputAsset"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { getContractCallInputs } from "@/utils/rpc.helpers"
jest.mock("@/utils/rpc.helpers", () => ({ getContractCallInputs: jest.fn() }))
;(global as any).logger = { info: jest.fn(), warn: jest.fn(), error: jest.fn() }

import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import { matchDeliveryTransfers, tryDecodeFillOrder, resolveFillEnrichment } from "@/utils/fill.helpers"
import { findUserOpHash } from "@/utils/userOp.helpers"

const USER_OPERATION_EVENT_TOPIC = "0x49628fd1471006c1482da88028e9ce4dbb080b815c9b0344d39e5a8e6ec1419f"
const ERC20_TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
const ORDER_FILLED_TOPIC = "0xdd5ba16ce7d9636800f5875b2a5572176a96f47947d3218b2a2ac4c5cc11f9bf"

const GATEWAY = "0x1111111111111111111111111111111111111111"
const ENTRY_POINT = "0x0000000071727de22e5e9d8baf0edac6f37da032"
const FILLER = "0x2222222222222222222222222222222222222222"
const BENEFICIARY = "0x3333333333333333333333333333333333333333"
const TOKEN_A = "0x4444444444444444444444444444444444444444"
const TOKEN_B = "0x5555555555555555555555555555555555555555"
const NATIVE = "0x0000000000000000000000000000000000000000"
const USER_OP_HASH = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

const pad32 = (address: string) => `0x${address.slice(2).padStart(64, "0")}`
const uint256 = (amount: bigint) => `0x${amount.toString(16).padStart(64, "0")}`

const transferLog = (logIndex: number, token: string, from: string, to: string, amount: bigint) =>
	({
		address: token,
		logIndex,
		topics: [ERC20_TRANSFER_TOPIC, pad32(from), pad32(to)],
		data: uint256(amount),
	}) as unknown as EthereumLog

const userOpLog = (logIndex: number, userOpHash: string, sender: string) =>
	({
		address: ENTRY_POINT,
		logIndex,
		topics: [USER_OPERATION_EVENT_TOPIC, userOpHash, pad32(sender), pad32(NATIVE)],
		data: "0x",
	}) as unknown as EthereumLog

const fillEventLog = (logIndex: number, gateway = GATEWAY) =>
	({
		address: gateway,
		logIndex,
		topics: [ORDER_FILLED_TOPIC, "0x" + "cc".repeat(32)],
		data: "0x",
	}) as unknown as EthereumLog

const fillLog = (logIndex: number) => ({ address: GATEWAY, logIndex, transactionHash: "0x" + "ee".repeat(32) })

describe("findUserOpHash", () => {
	it("returns the first UserOperationEvent after the fill whose sender is the filler", () => {
		const logs = [
			userOpLog(2, "0x" + "11".repeat(32), FILLER), // earlier op, before the fill
			fillEventLog(5),
			userOpLog(9, USER_OP_HASH, FILLER),
			userOpLog(12, "0x" + "33".repeat(32), FILLER), // later op
		]

		expect(findUserOpHash(logs, FILLER, 5)).toBe(USER_OP_HASH)
	})

	it("does not attribute a fill to a later operation after a different sender", () => {
		expect(
			findUserOpHash([userOpLog(7, USER_OP_HASH, BENEFICIARY), userOpLog(9, USER_OP_HASH, FILLER)], FILLER, 5),
		).toBeUndefined()
	})

	it("ignores spoofed events from non-EntryPoint contracts", () => {
		const spoof = { ...userOpLog(6, USER_OP_HASH, FILLER), address: TOKEN_A }
		expect(findUserOpHash([spoof], FILLER, 5)).toBeUndefined()
		expect(findUserOpHash([spoof, userOpLog(9, USER_OP_HASH, FILLER)], FILLER, 5)).toBe(USER_OP_HASH)
	})

	it("matches the filler address case-insensitively", () => {
		const logs = [userOpLog(9, USER_OP_HASH, FILLER)]

		expect(findUserOpHash(logs, FILLER.toUpperCase().replace("0X", "0x"), 5)).toBe(USER_OP_HASH)
	})

	it("returns undefined for plain EOA fills", () => {
		const logs = [fillEventLog(5), transferLog(3, TOKEN_A, FILLER, BENEFICIARY, 100n)]

		expect(findUserOpHash(logs, FILLER, 5)).toBeUndefined()
	})

	it("ignores UserOperationEvents emitted before the fill", () => {
		const logs = [userOpLog(2, USER_OP_HASH, FILLER), fillEventLog(5)]

		expect(findUserOpHash(logs, FILLER, 5)).toBeUndefined()
	})
})

describe("matchDeliveryTransfers", () => {
	const outputs = (...tokens: string[]) => tokens.map((token) => ({ token: pad32(token) as any, amount: 100n }))

	it("returns the actually transferred amount for each output", () => {
		const logs = [
			transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 105n), // overfill: 100 promised + 5 surplus share
			transferLog(3, TOKEN_B, FILLER, BENEFICIARY, 200n),
			fillEventLog(5),
		]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A, TOKEN_B))).toEqual([
			105n,
			200n,
		])
	})

	it("consumes transfers in log order for repeated tokens", () => {
		const logs = [
			transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 50n),
			transferLog(3, TOKEN_A, FILLER, BENEFICIARY, 60n),
			fillEventLog(5),
		]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A, TOKEN_A))).toEqual([
			50n,
			60n,
		])
	})

	it("records an ERC-20 zero-output slot without consuming a repeated-token transfer", () => {
		const logs = [transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 60n), fillEventLog(5)]
		const partialOutputs = [
			{ token: pad32(TOKEN_A) as any, amount: 0n },
			{ token: pad32(TOKEN_A) as any, amount: 60n },
		]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, partialOutputs)).toEqual([0n, 60n])
	})

	it("resolves native token outputs to undefined", () => {
		const logs = [transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 100n), fillEventLog(5)]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(NATIVE, TOKEN_A))).toEqual([
			undefined,
			100n,
		])
	})

	it("ignores transfers that are not filler to beneficiary", () => {
		const logs = [
			transferLog(1, TOKEN_A, FILLER, GATEWAY, 5n), // surplus protocol share
			transferLog(2, TOKEN_A, BENEFICIARY, FILLER, 7n), // wrong direction
			transferLog(3, TOKEN_A, FILLER, BENEFICIARY, 100n),
			fillEventLog(5),
			transferLog(6, TOKEN_A, FILLER, BENEFICIARY, 9n), // after the fill event
		]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([100n])
	})

	it("only considers transfers after the previous fill in a batched transaction", () => {
		const logs = [
			transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 100n), // belongs to the first fill
			fillEventLog(3),
			transferLog(6, TOKEN_A, FILLER, BENEFICIARY, 250n),
			fillEventLog(8),
		]

		expect(matchDeliveryTransfers(logs, fillLog(8), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([250n])
		expect(matchDeliveryTransfers(logs, fillLog(3), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([100n])
	})

	it("leaves ambiguous deliveries null instead of choosing an unrelated transfer", () => {
		const logs = [
			transferLog(1, TOKEN_A, FILLER, BENEFICIARY, 7n),
			transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 105n),
		]
		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([undefined])
	})

	it("does not shift repeated-token amounts when a delivery is missing", () => {
		expect(
			matchDeliveryTransfers(
				[transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 50n)],
				fillLog(5),
				FILLER,
				BENEFICIARY,
				outputs(TOKEN_A, TOKEN_A),
			),
		).toEqual([undefined, undefined])
	})

	it("ignores malformed Transfer data", () => {
		const log = { ...transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 100n), data: "0xgarbage" }
		expect(matchDeliveryTransfers([log], fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([undefined])
	})

	it("uses PartialFill logs as batch boundaries", () => {
		const boundary = {
			...fillEventLog(3),
			topics: [new Interface(IntentGatewayV3Abi).getEventTopic("PartialFill")],
		}
		const logs = [
			transferLog(2, TOKEN_A, FILLER, BENEFICIARY, 50n),
			boundary,
			transferLog(4, TOKEN_A, FILLER, BENEFICIARY, 60n),
		]
		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([60n])
	})

	it("resolves to undefined when no matching transfer exists", () => {
		const logs = [fillEventLog(5)]

		expect(matchDeliveryTransfers(logs, fillLog(5), FILLER, BENEFICIARY, outputs(TOKEN_A))).toEqual([undefined])
	})
})

describe("tryDecodeFillOrder", () => {
	const intentGatewayInterface = new Interface(IntentGatewayV3Abi)

	const order = {
		user: pad32(BENEFICIARY),
		source: "0x45564d2d3937", // "EVM-97"
		destination: "0x45564d2d3536", // "EVM-56"
		deadline: 1000n,
		nonce: 1n,
		fees: 5n,
		session: NATIVE,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: pad32(TOKEN_A), amount: 100n }],
		output: {
			beneficiary: pad32(BENEFICIARY),
			assets: [{ token: pad32(TOKEN_B), amount: 200n }],
			call: "0x",
		},
	}

	describe("receipt enrichment", () => {
		const encode = (value = order) =>
			intentGatewayInterface.encodeFunctionData("fillOrder", [
				value,
				{ relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 0n, outputs: value.output.assets },
			])
		const enrich = (input: string) =>
			resolveFillEnrichment(
				{
					...fillLog(5),
					transaction: {
						input,
						receipt: async () => ({
							logs: [
								transferLog(2, TOKEN_B, FILLER, BENEFICIARY, 205n),
								userOpLog(6, USER_OP_HASH, FILLER),
							],
						}),
					},
				} as any,
				{
					commitment: IntentGatewayV3Service.computeOrderCommitment(tryDecodeFillOrder(encode())!),
					filler: FILLER,
					outputs: [{ token: pad32(TOKEN_B) as any, amount: 200n }],
					chain: "EVM-56",
				},
			)
		beforeEach(() => {
			jest.spyOn(IOrderV3OutputAsset, "get").mockResolvedValue(undefined)
			jest.mocked(getContractCallInputs).mockReset()
		})
		afterEach(() => jest.restoreAllMocks())

		it("recovers an unindexed order directly without needing debug tracing", async () => {
			jest.mocked(getContractCallInputs).mockRejectedValue(new Error("tracing unavailable"))
			expect(await enrich(encode())).toEqual({ userOpHash: USER_OP_HASH, amountsReceived: [205n] })
			expect(getContractCallInputs).not.toHaveBeenCalled()
		})
		it("selects the matching commitment from multiple nested fill calls", async () => {
			jest.mocked(getContractCallInputs).mockResolvedValue([
				encode({ ...order, nonce: 2n }) as any,
				encode() as any,
			])
			expect(await enrich("0x12345678")).toEqual({ userOpHash: USER_OP_HASH, amountsReceived: [205n] })
			expect(getContractCallInputs).toHaveBeenCalledWith(fillLog(5).transactionHash, GATEWAY, "EVM-56")
		})
		it("rejects calldata for a different order", async () => {
			jest.mocked(getContractCallInputs).mockResolvedValue([encode({ ...order, nonce: 2n }) as any])
			expect(await enrich("0x12345678")).toEqual({ userOpHash: USER_OP_HASH, amountsReceived: undefined })
		})
		it("retains the userop hash when optional tracing is unavailable", async () => {
			jest.mocked(getContractCallInputs).mockRejectedValue(new Error("tracing unavailable"))
			expect(await enrich("0x12345678")).toEqual({ userOpHash: USER_OP_HASH, amountsReceived: undefined })
		})
		it("uses indexed beneficiaries without tracing", async () => {
			jest.mocked(IOrderV3OutputAsset.get).mockResolvedValue({ beneficiary: pad32(BENEFICIARY) } as any)
			expect(await enrich("0x12345678")).toEqual({ userOpHash: USER_OP_HASH, amountsReceived: [205n] })
			expect(getContractCallInputs).not.toHaveBeenCalled()
		})
	})

	it("decodes a fillOrder call", () => {
		const calldata = intentGatewayInterface.encodeFunctionData("fillOrder", [
			order,
			{
				relayerFee: 0n,
				nativeDispatchFee: 0n,
				validUntil: 0n,
				outputs: [{ token: pad32(TOKEN_B), amount: 200n }],
			},
		])

		const decoded = tryDecodeFillOrder(calldata)

		expect(decoded).not.toBeNull()
		expect(decoded!.user).toBe(pad32(BENEFICIARY))
		expect(decoded!.outputs.beneficiary).toBe(pad32(BENEFICIARY))
		expect(decoded!.outputs.assets).toEqual([{ token: pad32(TOKEN_B), amount: 200n }])
		expect(decoded!.inputs).toEqual([{ token: pad32(TOKEN_A), amount: 100n }])
		expect(decoded!.deadline).toBe(1000n)
		expect(decoded!.fees).toBe(5n)
	})

	it("decodes historical fills before FillOptions gained validUntil", () => {
		const fragment = intentGatewayInterface.getFunction("fillOrder").format("full")
		const legacy = new Interface([fragment.replace("uint256 validUntil, ", "")])
		const calldata = legacy.encodeFunctionData("fillOrder", [
			order,
			{ relayerFee: 0n, nativeDispatchFee: 0n, outputs: order.output.assets },
		])
		expect(tryDecodeFillOrder(calldata)?.outputs.beneficiary).toBe(pad32(BENEFICIARY))
	})

	it("returns null for other gateway calls", () => {
		const calldata = intentGatewayInterface.encodeFunctionData("placeOrder", [order, pad32(FILLER)])

		expect(tryDecodeFillOrder(calldata)).toBeNull()
	})

	it("returns null for foreign calldata", () => {
		expect(tryDecodeFillOrder("0x12345678")).toBeNull()
	})
})
