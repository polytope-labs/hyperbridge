import { describe, it, expect, vi } from "vitest"
import { slice, createPublicClient, custom, encodeFunctionData, type PublicClient } from "viem"
import { baseSepolia } from "viem/chains"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import {
	encodeFillOrder,
	decodeFillOrder,
	assertGatewayRelease,
	supportsRateFills,
	FILL_ORDER_SELECTOR,
} from "@/protocols/intents/fillOrderCodec"
import type { FillOptions, HexString, Order, TokenInfo } from "@/types"

const GATEWAY = "0x1111111111111111111111111111111111111111" as HexString
const TOKEN = "0x0000000000000000000000000000000000000000000000000000000000000002" as HexString

/** The pre-quote `fillOrder(Order, (relayerFee, nativeDispatchFee, validUntil, outputs))`, selector `0xa5470064`. */
const PRE_QUOTE_FILL_ORDER_ABI = [
	{
		type: "function",
		name: "fillOrder",
		stateMutability: "payable",
		outputs: [],
		inputs: [
			IntentGatewayV2ABI.find((e) => e.type === "function" && e.name === "fillOrder")!.inputs![0],
			{
				name: "options",
				type: "tuple",
				components: [
					{ name: "relayerFee", type: "uint256" },
					{ name: "nativeDispatchFee", type: "uint256" },
					{ name: "validUntil", type: "uint256" },
					{
						name: "outputs",
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
						],
					},
				],
			},
		],
	},
] as const

function order(): Order {
	return {
		user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
		source: "0x",
		destination: "0x",
		deadline: 100n,
		nonce: 1n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 1_000n }],
		output: {
			beneficiary: ("0x" + "00".repeat(32)) as HexString,
			assets: [{ token: TOKEN, amount: 0n }],
			call: "0x",
		},
	} as unknown as Order
}

function options(validUntil: bigint, inputs: TokenInfo[] = order().inputs): FillOptions {
	return { relayerFee: 0n, nativeDispatchFee: 0n, validUntil, outputs: [{ token: TOKEN, amount: 500n }], inputs }
}

function client(readContract: any, chainId = 8453) {
	return { chain: { id: chainId }, readContract } as any
}

describe("encodeFillOrder", () => {
	it("emits the pinned selector and round-trips the quote", () => {
		const inputs: TokenInfo[] = [{ token: TOKEN, amount: 400n }]
		const data = encodeFillOrder(order(), options(77n, inputs))

		expect(slice(data, 0, 4)).toBe(FILL_ORDER_SELECTOR)
		expect(decodeFillOrder(data)).toEqual({ order: order(), options: options(77n, inputs) })
	})

	it("requires a take for every leg", () => {
		expect(() => encodeFillOrder(order(), { ...options(7n), inputs: undefined } as unknown as FillOptions)).toThrow(
			/inputs|quote/i,
		)
		expect(() => encodeFillOrder(order(), options(7n, []))).toThrow(/inputs|quote/i)
	})

	it("rejects a take whose token is not the leg's input", () => {
		const other = "0x0000000000000000000000000000000000000000000000000000000000000003" as HexString
		expect(() => encodeFillOrder(order(), options(7n, [{ token: other, amount: 1n }]))).toThrow(/token/i)
	})

	it("rejects a leg quoted on one side only", () => {
		expect(() => encodeFillOrder(order(), options(7n, [{ token: TOKEN, amount: 0n }]))).toThrow(/zero/i)
	})
})

describe("decodeFillOrder", () => {
	it("returns null for calldata that is not a fillOrder call", () => {
		expect(decodeFillOrder("0xdeadbeef" as HexString)).toBeNull()
	})

	it("returns null for the pre-quote fillOrder shape", () => {
		const { inputs: _inputs, ...preQuote } = options(9n)
		const data = encodeFunctionData({
			abi: PRE_QUOTE_FILL_ORDER_ABI,
			functionName: "fillOrder",
			args: [order() as any, preQuote as any],
		}) as HexString

		expect(slice(data, 0, 4)).toBe("0xa5470064")
		expect(decodeFillOrder(data)).toBeNull()
	})

	it("returns null for a fillOrder whose quotes do not cover the legs", () => {
		const data = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [order() as any, options(9n, []) as any],
		}) as HexString

		expect(slice(data, 0, 4)).toBe(FILL_ORDER_SELECTOR)
		expect(decodeFillOrder(data)).toBeNull()
	})
})

describe("assertGatewayRelease", () => {
	it("accepts release 3", async () => {
		await expect(assertGatewayRelease(client(vi.fn().mockResolvedValue(3n)), GATEWAY)).resolves.toBeUndefined()
	})

	it.each([0n, 1n, 2n, 4n, 5n, (1n << 64n) - 1n])("rejects release %s", async (release) => {
		await expect(assertGatewayRelease(client(vi.fn().mockResolvedValue(release)), GATEWAY)).rejects.toThrow(
			/release 3/,
		)
	})

	it("rejects a malformed version instead of guessing", async () => {
		await expect(assertGatewayRelease(client(vi.fn().mockResolvedValue("0x12345678")), GATEWAY)).rejects.toThrow(
			/release 3/,
		)
	})

	it("is never cached, so an upgrade is seen on the next read", async () => {
		const c = client(vi.fn().mockResolvedValue(2n))
		await expect(assertGatewayRelease(c, GATEWAY)).rejects.toThrow(/release 2/)
		c.readContract.mockResolvedValue(3n)
		await expect(assertGatewayRelease(c, GATEWAY)).resolves.toBeUndefined()
		expect(c.readContract).toHaveBeenCalledTimes(2)
	})
})

describe("supportsRateFills", () => {
	it("reads only the gateway release", async () => {
		const readContract = vi.fn().mockResolvedValue(3n)
		await expect(supportsRateFills(client(readContract), GATEWAY)).resolves.toBe(true)
		expect(readContract).toHaveBeenCalledTimes(1)
		expect(readContract.mock.calls[0][0].address).toBe(GATEWAY)
	})

	it.each([0n, 1n, 2n, 4n, 5n, (1n << 64n) - 1n])("rejects gateway release %s", async (release) => {
		await expect(supportsRateFills(client(vi.fn().mockResolvedValue(release)), GATEWAY)).resolves.toBe(false)
	})

	it("does not accept the old boolean marker", async () => {
		await expect(supportsRateFills(client(vi.fn().mockResolvedValue(true)), GATEWAY)).resolves.toBe(false)
	})

	it("does not cache capability across calls", async () => {
		const c = client(vi.fn().mockResolvedValue(3n))

		expect(await supportsRateFills(c, GATEWAY)).toBe(true)
		c.readContract.mockResolvedValue(2n)
		expect(await supportsRateFills(c, GATEWAY)).toBe(false)
		expect(c.readContract).toHaveBeenCalledTimes(2)
	})
})

describe("version() failures with real viem errors", () => {
	function rpcClient(rpcError?: { code: number; message: string; data?: string }) {
		const request = vi.fn(async ({ method }: { method: string }) => {
			if (method !== "eth_call") throw new Error(`Unexpected RPC method: ${method}`)
			if (rpcError) throw Object.assign(new Error(rpcError.message), rpcError)
			return "0x"
		})
		return createPublicClient({
			chain: baseSepolia,
			transport: custom({ request }, { retryCount: 0 }),
		}) as unknown as PublicClient
	}

	it.each([
		{ code: -32603, message: "upstream request timeout" },
		{ code: -32603, message: "upstream request timeout", data: "0x" },
		{ code: -32601, message: "method not found" },
		{ code: -32005, message: "rate limit exceeded" },
	])("propagates provider failure $code: $message", async (rpcError) => {
		const c = rpcClient(rpcError)
		await expect(assertGatewayRelease(c, GATEWAY)).rejects.toThrow(rpcError.message)
		await expect(supportsRateFills(c, GATEWAY)).rejects.toThrow(rpcError.message)
	})

	it.each([
		{ code: 3, message: "execution reverted", data: "0x" },
		{ code: -32000, message: "execution reverted", data: "0x" },
		{ code: -32603, message: "execution reverted", data: "0x" },
		{ code: -32000, message: "function selector was not recognized and there's no fallback function" },
	])("treats a genuine EVM failure $code: $message as a missing getter", async (rpcError) => {
		const c = rpcClient(rpcError)
		await expect(assertGatewayRelease(c, GATEWAY)).rejects.toThrow(/no version\(\)/)
		await expect(supportsRateFills(c, GATEWAY)).resolves.toBe(false)
	})

	it("treats a successful call returning no data as a missing getter", async () => {
		const c = rpcClient()
		await expect(assertGatewayRelease(c, GATEWAY)).rejects.toThrow(/no version\(\)/)
		await expect(supportsRateFills(c, GATEWAY)).resolves.toBe(false)
	})
})
