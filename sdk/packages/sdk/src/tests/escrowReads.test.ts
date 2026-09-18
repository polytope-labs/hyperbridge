import { readLegEscrow, readLegPartialFill } from "@/protocols/intents/escrowReads"
import type { HexString } from "@/types"
import {
	RpcRequestError,
	createPublicClient,
	custom,
	decodeFunctionData,
	encodeAbiParameters,
	parseAbi,
	toFunctionSelector,
} from "viem"
import { describe, expect, it } from "vitest"

const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA" as HexString
const COMMITMENT = `0x${"ab".repeat(32)}` as HexString
const USDC = "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" as HexString

const GETTERS = parseAbi([
	"function _orders(bytes32, uint256) view returns (uint256)",
	"function _orders(bytes32, address) view returns (uint256)",
	"function _partialFills(bytes32, uint256) view returns (uint256)",
	"function _partialFills(bytes32, bytes32) view returns (uint256)",
])

const LEG = [toFunctionSelector("_orders(bytes32,uint256)"), toFunctionSelector("_partialFills(bytes32,uint256)")]
const TOKEN = [toFunctionSelector("_orders(bytes32,address)"), toFunctionSelector("_partialFills(bytes32,bytes32)")]

/**
 * A client over a gateway that answers only one keying's getters, reverting on the other selectors the
 * way an implementation without them does. `values` maps the getter's key argument to the stored value.
 */
function gatewayClient(state: { keying: "leg" | "token" | "down"; values: Record<string, bigint>; calls: string[] }) {
	return createPublicClient({
		transport: custom(
			{
				async request({ method, params }) {
					if (method !== "eth_call") throw new Error(`unexpected ${method}`)
					const data = (params as [{ data: HexString }])[0].data
					const selector = data.slice(0, 10)
					state.calls.push(selector)
					if (state.keying === "down") throw new Error("fetch failed")
					const answers = state.keying === "leg" ? LEG : TOKEN
					if (!answers.includes(selector)) {
						// What an HTTP RPC returns for a call to a selector the implementation lacks.
						throw new RpcRequestError({
							body: {},
							error: { code: 3, message: "execution reverted", data: "0x" },
							url: "https://rpc.example",
						})
					}
					const { args } = decodeFunctionData({ abi: GETTERS, data })
					const key = String(args?.[1]).toLowerCase()
					return encodeAbiParameters([{ type: "uint256" }], [state.values[key] ?? 0n])
				},
			},
			{ retryCount: 0 },
		),
	})
}

describe("readLegEscrow", () => {
	it("reads a per-leg gateway by index", async () => {
		const state = { keying: "leg" as const, values: { "1": 700n }, calls: [] as string[] }
		const client = gatewayClient(state)

		expect(await readLegEscrow(client, GATEWAY, COMMITMENT, 1, USDC)).toBe(700n)
		expect(state.calls).toEqual([LEG[0]])
	})

	it("falls back to the token getter on a gateway without per-leg escrow and remembers it", async () => {
		const state = {
			keying: "token" as const,
			values: { "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": 1000n },
			calls: [] as string[],
		}
		const client = gatewayClient(state)

		expect(await readLegEscrow(client, GATEWAY, COMMITMENT, 0, USDC)).toBe(1000n)
		expect(state.calls).toEqual([LEG[0], TOKEN[0]])

		state.calls.length = 0
		expect(await readLegEscrow(client, GATEWAY, COMMITMENT, 0, USDC)).toBe(1000n)
		expect(state.calls).toEqual([TOKEN[0]])
	})

	it("switches back once the gateway is upgraded", async () => {
		const state = {
			keying: "token" as "leg" | "token",
			values: { "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": 1000n, "0": 400n } as Record<string, bigint>,
			calls: [] as string[],
		}
		const client = gatewayClient(state)
		await readLegEscrow(client, GATEWAY, COMMITMENT, 0, USDC)

		state.keying = "leg"
		state.calls.length = 0
		expect(await readLegEscrow(client, GATEWAY, COMMITMENT, 0, USDC)).toBe(400n)
		expect(state.calls).toEqual([TOKEN[0], LEG[0]])
	})

	it("does not fall back on a transport error", async () => {
		const state = { keying: "down" as const, values: {}, calls: [] as string[] }
		const client = gatewayClient(state)

		await expect(readLegEscrow(client, GATEWAY, COMMITMENT, 0, USDC)).rejects.toThrow("fetch failed")
		expect(state.calls).toEqual([LEG[0]])
	})
})

describe("readLegPartialFill", () => {
	it("reads per leg, or by bytes32 token on a gateway without per-leg escrow", async () => {
		const leg = { keying: "leg" as const, values: { "1": 990n }, calls: [] as string[] }
		expect(await readLegPartialFill(gatewayClient(leg), GATEWAY, COMMITMENT, 1, USDC)).toBe(990n)
		expect(leg.calls).toEqual([LEG[1]])

		const token = { keying: "token" as const, values: { [USDC]: 250n }, calls: [] as string[] }
		expect(await readLegPartialFill(gatewayClient(token), GATEWAY, COMMITMENT, 0, USDC)).toBe(250n)
		expect(token.calls).toEqual([LEG[1], TOKEN[1]])
	})
})
