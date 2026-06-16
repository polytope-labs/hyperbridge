import { describe, it, expect } from "vitest"
import { encodeFunctionData, type PublicClient, type Hex } from "viem"
import { getContractCallInputs, getContractCallInput, type HexString } from "@/utils"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { EvmChain } from "@/chains/evm"

const GATEWAY = "0x000000000000000000000000000000000000ca11"
const OTHER = "0x0000000000000000000000000000000000000bee"
const TX_HASH = ("0x" + "ab".repeat(32)) as HexString

const GRAFFITI = ("0x" + "00".repeat(32)) as Hex
const USER = ("0x" + "11".repeat(32)) as Hex
const TOKEN_IN = ("0x" + "aa".repeat(32)) as Hex
const TOKEN_OUT = ("0x" + "bb".repeat(32)) as Hex
const BENEFICIARY = ("0x" + "cc".repeat(32)) as Hex

function makeOrderStruct(nonce: bigint) {
	return {
		user: USER,
		source: "0x45564d2d31" as Hex, // "EVM-1"
		destination: "0x45564d2d3130" as Hex, // "EVM-10"
		deadline: 1700000000n,
		nonce,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as Hex,
		predispatch: { assets: [], call: "0x" as Hex },
		inputs: [{ token: TOKEN_IN, amount: 100n }],
		output: {
			beneficiary: BENEFICIARY,
			assets: [{ token: TOKEN_OUT, amount: 100n }],
			call: "0x" as Hex,
		},
	}
}

function placeOrderCalldata(nonce: bigint): HexString {
	return encodeFunctionData({
		abi: IntentGatewayV2ABI,
		functionName: "placeOrder",
		args: [makeOrderStruct(nonce) as any, GRAFFITI],
	}) as HexString
}

/**
 * Builds a stub PublicClient.extend / debug_traceTransaction pipeline that
 * surfaces a fixed CallTracerCall fixture to the code under test.
 */
function stubClient(trace: any): PublicClient {
	const request = async ({ method }: { method: string }) => {
		if (method === "debug_traceTransaction") return trace
		throw new Error(`unexpected method: ${method}`)
	}
	const client: any = {
		request,
		extend(decorator: (c: any) => any) {
			return { ...client, ...decorator(client) }
		},
	}
	return client as PublicClient
}

describe("getContractCallInputs", () => {
	it("returns every call to the target in execution order", async () => {
		const trace = {
			from: "0xabc",
			to: GATEWAY,
			input: "0xtop",
			calls: [
				{ from: GATEWAY, to: OTHER, input: "0x01", calls: [] },
				{ from: "0xabc", to: GATEWAY, input: "0x02", calls: [] },
				{
					from: "0xabc",
					to: OTHER,
					input: "0x03",
					calls: [{ from: OTHER, to: GATEWAY, input: "0x04", calls: [] }],
				},
			],
		}

		const inputs = await getContractCallInputs(stubClient(trace), TX_HASH, GATEWAY)

		expect(inputs).toEqual(["0xtop", "0x02", "0x04"])
	})

	it("returns an empty list when the target is not called", async () => {
		const trace = {
			from: "0xabc",
			to: OTHER,
			input: "0x00",
			calls: [{ from: "0xabc", to: OTHER, input: "0x01", calls: [] }],
		}

		const inputs = await getContractCallInputs(stubClient(trace), TX_HASH, GATEWAY)

		expect(inputs).toEqual([])
	})

	it("getContractCallInput (legacy wrapper) returns the first match", async () => {
		const trace = {
			from: "0xabc",
			to: OTHER,
			input: "0x00",
			calls: [
				{ from: "0xabc", to: GATEWAY, input: "0xfirst", calls: [] },
				{ from: "0xabc", to: GATEWAY, input: "0xsecond", calls: [] },
			],
		}

		const result = await getContractCallInput(stubClient(trace), TX_HASH, GATEWAY)

		expect(result).toBe("0xfirst")
	})
})

describe("EvmChain.getPlaceOrderCalldata", () => {
	function newChain(trace: any): EvmChain {
		const chain = Object.create(EvmChain.prototype) as EvmChain
		;(chain as any).publicClient = stubClient(trace)
		return chain
	}

	it("returns the K-th placeOrder calldata in execution order", async () => {
		const order0 = placeOrderCalldata(1n)
		const order1 = placeOrderCalldata(2n)

		const trace = {
			from: "0xabc",
			to: "0xabc",
			input: "0x",
			calls: [
				{ from: "0xabc", to: GATEWAY, input: order0, calls: [] },
				{ from: "0xabc", to: OTHER, input: "0xnoise", calls: [] },
				{ from: "0xabc", to: GATEWAY, input: order1, calls: [] },
			],
		}

		const chain = newChain(trace)
		const first = await chain.getPlaceOrderCalldata(TX_HASH, GATEWAY, 0)
		const second = await chain.getPlaceOrderCalldata(TX_HASH, GATEWAY, 1)

		expect(first).toBe(order0)
		expect(second).toBe(order1)
	})

	it("defaults occurrenceIndex to 0", async () => {
		const order0 = placeOrderCalldata(7n)
		const trace = {
			from: "0xabc",
			to: GATEWAY,
			input: order0,
			calls: [],
		}

		const chain = newChain(trace)
		const result = await chain.getPlaceOrderCalldata(TX_HASH, GATEWAY)
		expect(result).toBe(order0)
	})

	it("throws when occurrenceIndex is out of range", async () => {
		const order0 = placeOrderCalldata(1n)
		const trace = {
			from: "0xabc",
			to: GATEWAY,
			input: order0,
			calls: [],
		}

		const chain = newChain(trace)
		await expect(chain.getPlaceOrderCalldata(TX_HASH, GATEWAY, 1)).rejects.toThrow(/out of range/)
	})

	it("throws when the gateway is not called at all", async () => {
		const trace = {
			from: "0xabc",
			to: "0xabc",
			input: "0x",
			calls: [{ from: "0xabc", to: OTHER, input: "0xnoise", calls: [] }],
		}

		const chain = newChain(trace)
		await expect(chain.getPlaceOrderCalldata(TX_HASH, GATEWAY, 0)).rejects.toThrow(/Failed to extract placeOrder/)
	})
})
