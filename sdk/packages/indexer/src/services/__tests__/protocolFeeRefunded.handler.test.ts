import { Interface } from "@ethersproject/abi"
import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import { handleDustCollectedEventV3 } from "@/handlers/events/intentGatewayV3/dustCollected.event.handler"
import { handleProtocolFeeRefundedEventV3 } from "@/handlers/events/intentGatewayV3/protocolFeeRefunded.event.handler"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import PriceHelper from "@/utils/price.helpers"

jest.mock("@/utils/rpc.helpers", () => ({ getBlockTimestamp: async () => 1_700n }))
jest.mock("@/utils/substrate.helpers", () => ({ getHostStateMachine: () => "EVM-8453" }))
jest.mock("@/utils/price.helpers", () => ({
	__esModule: true,
	default: { getTokenPriceInUSDUniswap: jest.fn() },
}))

const records = new Map<string, any>()
const abi = new Interface(IntentGatewayV3Abi)
const commitment = `0x${"11".repeat(32)}`
const transactionHash = `0x${"22".repeat(32)}`
const blockHash = `0x${"33".repeat(32)}`
const nativeToken = "0x0000000000000000000000000000000000000000"
const otherToken = "0x4444444444444444444444444444444444444444"

function event(name: "ProtocolFeeRefunded" | "DustCollected", args: unknown[], logIndex: number) {
	const encoded = abi.encodeEventLog(abi.getEvent(name), args)
	return {
		blockNumber: 99,
		transactionHash,
		blockHash,
		logIndex,
		args: abi.decodeEventLog(name, encoded.data, encoded.topics),
	} as any
}

beforeEach(() => {
	records.clear()
	;(global as any).chainId = "8453"
	;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
	;(global as any).store = {
		get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
		set: jest.fn(async (entity: string, id: string, data: any) => records.set(`${entity}:${id}`, { ...data })),
	}
	jest.mocked(PriceHelper.getTokenPriceInUSDUniswap).mockResolvedValue({ amountValueInUSD: "60" } as any)
})

it("decodes and records protocol fee attribution without changing the order", async () => {
	records.set(`IOrderV3:${commitment}`, { id: commitment, status: "PLACED" })

	await handleProtocolFeeRefundedEventV3(event("ProtocolFeeRefunded", [commitment, otherToken, 40n], 7))

	expect(records.get(`IOrderV3ProtocolFeeRefund:${transactionHash}.7`)).toEqual({
		id: `${transactionHash}.7`,
		orderId: commitment,
		chain: "8453",
		token: otherToken,
		amount: 40n,
		timestamp: 1_700n,
		blockNumber: "99",
		transactionHash,
		createdAt: new Date(1_700_000),
	})
	expect(records.get(`IOrderV3:${commitment}`)).toEqual({ id: commitment, status: "PLACED" })
	expect((global as any).store.set.mock.calls.some(([entity]: string[]) => entity === "IOrderV3")).toBe(false)
})

it("records a protocol fee refund even when the order has not arrived", async () => {
	await IntentGatewayV3Service.recordProtocolFeeRefund(commitment, otherToken, 40n, {
		transactionHash,
		blockNumber: 99,
		timestamp: 1_700n,
		logIndex: 7,
	})

	expect(records.get(`IOrderV3ProtocolFeeRefund:${transactionHash}.7`)).toMatchObject({
		orderId: commitment,
		chain: "8453",
		token: otherToken,
		amount: 40n,
	})
	expect(records.has(`IOrderV3:${commitment}`)).toBe(false)
})

it("keeps the first immutable record when the same refund log is replayed", async () => {
	const log = { transactionHash, blockNumber: 99, timestamp: 1_700n, logIndex: 7 }
	await IntentGatewayV3Service.recordProtocolFeeRefund(commitment, otherToken, 40n, log)
	await IntentGatewayV3Service.recordProtocolFeeRefund(commitment, nativeToken, 999n, log)

	expect(records.get(`IOrderV3ProtocolFeeRefund:${transactionHash}.7`)).toMatchObject({
		token: otherToken,
		amount: 40n,
	})
	expect(
		(global as any).store.set.mock.calls.filter(([entity]: string[]) => entity === "IOrderV3ProtocolFeeRefund"),
	).toHaveLength(1)
})

it("records a refund separately and recognizes only the settled DustCollected amount as revenue", async () => {
	await handleProtocolFeeRefundedEventV3(event("ProtocolFeeRefunded", [commitment, nativeToken, 40n], 7))
	await handleDustCollectedEventV3(event("DustCollected", [nativeToken, 60n], 8))

	expect(records.get(`IOrderV3ProtocolFeeRefund:${transactionHash}.7`).amount).toBe(40n)
	expect(records.get("ProtocolDustCollected:EVM-8453-0x0000000000000000000000000000000000000000").amount).toBe(60n)
	expect(records.get("CumulativeDustCollectedPerChain:EVM-8453").amountUSD).toBe(60n * 10n ** 18n)
})
