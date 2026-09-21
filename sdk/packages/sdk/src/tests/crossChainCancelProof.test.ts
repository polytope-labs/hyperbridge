import { concatHex, decodeAbiParameters } from "viem"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { OrderCanceller } from "@/protocols/intents/OrderCanceller"
import { OrderStatusChecker } from "@/protocols/intents/OrderStatusChecker"
import { partialFillSlot } from "@/protocols/intents/escrowReads"
import * as intentUtils from "@/protocols/intents/utils"
import type { HexString, IGetRequest, Order } from "@/types"

vi.mock("@/protocols/intents/utils", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/protocols/intents/utils")>()),
	convertGasToFeeToken: vi.fn(),
}))

const COMMITMENT = `0x${"66".repeat(32)}` as HexString
const GATEWAY = "0x9876543210987654321098765432109876543210" as HexString
const USER = "0xea4f68301acec0dc9bbe10f15730c59fb79d237e" as HexString
const USER_BYTES32 = `0x${"00".repeat(12)}${USER.slice(2)}` as HexString
const TOKEN_A = `0x${"00".repeat(12)}${"aa".repeat(20)}` as HexString
const TOKEN_B = `0x${"00".repeat(12)}${"bb".repeat(20)}` as HexString

// keccak256(index ++ keccak256(commitment ++ 11)), computed independently with `cast keccak`.
const SLOT_0 = "0x5542f3031c66a96543f34071988f40fa3c7a31c736c9b3fa5e718d9051f8c300" as HexString
const SLOT_1 = "0x2fa6ee34535e322252e38486728f533a20445176bb8e9b9a0c1b6deead79c730" as HexString

const order: Order = {
	id: COMMITMENT,
	user: USER,
	source: "EVM-1",
	destination: "EVM-42161",
	deadline: 100n,
	nonce: 0n,
	fees: 0n,
	session: "0x0000000000000000000000000000000000000000",
	predispatch: { assets: [], call: "0x" },
	inputs: [
		{ token: TOKEN_A, amount: 1000n },
		{ token: TOKEN_B, amount: 500n },
	],
	output: {
		beneficiary: USER_BYTES32,
		assets: [
			{ token: TOKEN_A, amount: 990n },
			{ token: TOKEN_B, amount: 495n },
		],
		call: "0x",
	},
}

describe("partialFillSlot", () => {
	it("derives the gateway's _partialFills[commitment][index] slot", () => {
		expect(partialFillSlot(COMMITMENT, 0)).toBe(SLOT_0)
		expect(partialFillSlot(COMMITMENT, 1)).toBe(SLOT_1)
	})
})

describe("cross-chain cancellation proof", () => {
	beforeEach(() => {
		vi.mocked(intentUtils.convertGasToFeeToken).mockReset()
		vi.mocked(intentUtils.convertGasToFeeToken).mockResolvedValue(1_000n)
	})

	it("quotes the GET the source gateway dispatches: one key per leg and its context", async () => {
		const quoteNative = vi.fn(async (_request: IGetRequest, _fee: bigint) => 10_000n)
		const ctx = {
			source: {
				configService: { getIntentGatewayAddress: () => GATEWAY },
				getHostNonce: async () => 1n,
				quoteNative,
			},
			dest: { configService: { getIntentGatewayAddress: () => GATEWAY } },
		}

		await new OrderCanceller(ctx as never).quoteCancelOrder(order)

		const request = quoteNative.mock.calls[0][0]
		expect(request.keys).toEqual([concatHex([GATEWAY, SLOT_0]), concatHex([GATEWAY, SLOT_1])])
		expect(request.height).toBe(order.deadline + 1n)
		const [commitment, user, inputs, totalRequired] = decodeAbiParameters(
			[
				{ type: "bytes32" },
				{ type: "bytes32" },
				{
					type: "tuple[]",
					components: [
						{ name: "token", type: "bytes32" },
						{ name: "amount", type: "uint256" },
					],
				},
				{ type: "uint256[]" },
			],
			request.context,
		)
		expect(commitment).toBe(COMMITMENT)
		expect(user).toBe(USER_BYTES32)
		expect(inputs).toEqual(order.inputs)
		expect(totalRequired).toEqual([990n, 495n])
	})

	it("proves every leg's _partialFills slot on the destination", async () => {
		const queryStateProof = vi.fn(async () => "0xproof" as HexString)
		const ctx = {
			dest: {
				config: { stateMachineId: "EVM-42161", consensusStateId: "ETH0" },
				configService: { getIntentGatewayAddress: () => GATEWAY },
				queryStateProof,
			},
		}
		const indexerClient = {
			hyperbridge: { config: { stateMachineId: "KUSAMA-4009" } },
			queryLatestStateMachineHeight: async () => 101n,
		}

		const stream = (new OrderCanceller(ctx as never) as any).fetchDestinationProof(order, indexerClient)
		const first = await stream.next()

		expect(first.value).toMatchObject({
			status: "DESTINATION_FINALIZED",
			proof: { height: 101n, proof: "0xproof" },
		})
		expect(queryStateProof).toHaveBeenCalledWith(101n, [SLOT_0, SLOT_1], GATEWAY)
	})
})

describe("OrderStatusChecker", () => {
	function checker(filled: HexString, credited: bigint[]) {
		const readContract = vi.fn(async ({ functionName, args }: { functionName: string; args: unknown[] }) =>
			functionName === "_filled" ? filled : credited[Number(args[1])],
		)
		const ctx = { dest: { configService: { getIntentGatewayAddress: () => GATEWAY }, client: { readContract } } }
		return { status: new OrderStatusChecker(ctx as never), readContract }
	}

	it("reads finality from the _filled getter", async () => {
		expect(await checker(`0x${"00".repeat(20)}`, []).status.isOrderFilled(order)).toBe(false)
		expect(await checker(USER, []).status.isOrderFilled(order)).toBe(true)
	})

	it("reports credited output per leg for a partially filled order", async () => {
		const { status } = checker(`0x${"00".repeat(20)}`, [400n, 0n])
		expect(await status.getFillProgress(order)).toEqual([
			{ token: TOKEN_A, amount: 400n },
			{ token: TOKEN_B, amount: 0n },
		])
		expect(await status.isOrderFilled(order)).toBe(false)
	})
})
