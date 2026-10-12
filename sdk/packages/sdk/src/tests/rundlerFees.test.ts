import { afterEach, describe, expect, it, vi } from "vitest"
import {
	applyRundlerPriorityFee,
	fetchRundlerPriorityFee,
	fetchRundlerSuggestedFees,
	rundlerUserOperationFees,
} from "@/protocols/intents/rundlerFees"

function stubBundler(reply: () => Promise<unknown>) {
	const fetch = vi.fn(async () => ({ json: reply }))
	vi.stubGlobal("fetch", fetch)
	return fetch
}

afterEach(() => vi.unstubAllGlobals())

describe("fetchRundlerPriorityFee", () => {
	it("returns the priority fee rundler requires", async () => {
		const fetch = stubBundler(async () => ({ jsonrpc: "2.0", id: 1, result: "0x6fc23ac00" }))

		expect(await fetchRundlerPriorityFee("https://rundler-a.example")).toBe(30_000_000_000n)
		const [url, init] = fetch.mock.calls[0] as unknown as [string, { body: string }]
		expect(url).toBe("https://rundler-a.example")
		expect(JSON.parse(init.body)).toMatchObject({ method: "rundler_maxPriorityFeePerGas", params: [] })
	})

	it("stops asking a bundler that does not serve the method", async () => {
		const fetch = stubBundler(async () => ({
			jsonrpc: "2.0",
			id: 1,
			error: { code: -32601, message: "Method not found" },
		}))

		expect(await fetchRundlerPriorityFee("https://other-a.example")).toBeNull()
		expect(await fetchRundlerPriorityFee("https://other-a.example")).toBeNull()
		expect(fetch).toHaveBeenCalledOnce()
	})

	it("asks again after an internal error", async () => {
		const fetch = stubBundler(async () => ({
			jsonrpc: "2.0",
			id: 1,
			error: { code: -32603, message: "should get required fees" },
		}))

		expect(await fetchRundlerPriorityFee("https://rundler-b.example")).toBeNull()
		expect(await fetchRundlerPriorityFee("https://rundler-b.example")).toBeNull()
		expect(fetch).toHaveBeenCalledTimes(2)
	})

	it("asks again after a network failure", async () => {
		const fetch = vi.fn().mockRejectedValue(new Error("ECONNREFUSED"))
		vi.stubGlobal("fetch", fetch)

		expect(await fetchRundlerPriorityFee("https://rundler-c.example")).toBeNull()
		expect(await fetchRundlerPriorityFee("https://rundler-c.example")).toBeNull()
		expect(fetch).toHaveBeenCalledTimes(2)
	})
})

describe("applyRundlerPriorityFee", () => {
	const bumps = { priorityFeeBumpPercent: 8n, maxFeeBumpPercent: 10n }

	it("raises both fees to the rundler floor when it is above the estimate", () => {
		const fees = applyRundlerPriorityFee(
			{ maxFeePerGas: 1_100_000n, maxPriorityFeePerGas: 1_080_000n },
			{ rundlerPriorityFee: 100_000_000n, baseFeePerGas: 900_000n, ...bumps },
		)

		expect(fees).toEqual({ maxPriorityFeePerGas: 108_000_000n, maxFeePerGas: 990_000n + 108_000_000n })
	})

	it("keeps fees that already clear the floor", () => {
		const fees = applyRundlerPriorityFee(
			{ maxFeePerGas: 1_100_000n, maxPriorityFeePerGas: 1_080_000n },
			{ rundlerPriorityFee: 1_000n, baseFeePerGas: 900_000n, ...bumps },
		)

		expect(fees).toEqual({ maxPriorityFeePerGas: 1_080_000n, maxFeePerGas: 1_100_000n })
	})
})

describe("fetchRundlerSuggestedFees", () => {
	it("returns the fees rundler suggests", async () => {
		const fetch = stubBundler(async () => ({
			jsonrpc: "2.0",
			id: 1,
			result: {
				priorityFee: "0x2faf080",
				baseFee: "0x96e3af8",
				blockNumber: "0x18f33c1",
				suggested: { maxPriorityFeePerGas: "0x3dfd240", maxFeePerGas: "0x15d6ee1b" },
			},
		}))

		expect(await fetchRundlerSuggestedFees("https://rundler-d.example")).toEqual({
			maxPriorityFeePerGas: 65_000_000n,
			maxFeePerGas: 366_407_195n,
		})
		const [, init] = fetch.mock.calls[0] as unknown as [string, { body: string }]
		expect(JSON.parse(init.body)).toMatchObject({ method: "rundler_getUserOperationGasPrice", params: [] })
	})

	it("stops asking a bundler that does not serve the method", async () => {
		const fetch = stubBundler(async () => ({
			jsonrpc: "2.0",
			id: 1,
			error: { code: -32601, message: "Method not found" },
		}))

		expect(await fetchRundlerSuggestedFees("https://other-b.example")).toBeNull()
		expect(await fetchRundlerSuggestedFees("https://other-b.example")).toBeNull()
		expect(fetch).toHaveBeenCalledOnce()
	})
})

describe("rundlerUserOperationFees", () => {
	const fees = { maxFeePerGas: 1_100_000n, maxPriorityFeePerGas: 1_080_000n }
	const params = { baseFeePerGas: 900_000n, priorityFeeBumpPercent: 8n, maxFeeBumpPercent: 10n }

	it("uses the suggested fees raised by the bumps, even below the estimate", async () => {
		stubBundler(async () => ({
			jsonrpc: "2.0",
			id: 1,
			result: { suggested: { maxPriorityFeePerGas: "0x186a0", maxFeePerGas: "0x16e360" } },
		}))

		expect(await rundlerUserOperationFees("https://rundler-e.example", fees, params)).toEqual({
			maxPriorityFeePerGas: 108_000n,
			maxFeePerGas: 1_650_000n,
		})
	})

	it("falls back to the required priority fee when the bundler suggests none", async () => {
		const fetch = vi.fn(async (_url: string, init: { body: string }) => {
			const { method } = JSON.parse(init.body) as { method: string }
			return {
				json: async () =>
					method === "rundler_maxPriorityFeePerGas"
						? { jsonrpc: "2.0", id: 1, result: "0x5f5e100" }
						: { jsonrpc: "2.0", id: 1, error: { code: -32601, message: "Method not found" } },
			}
		})
		vi.stubGlobal("fetch", fetch)

		expect(await rundlerUserOperationFees("https://rundler-f.example", fees, params)).toEqual({
			maxPriorityFeePerGas: 108_000_000n,
			maxFeePerGas: 990_000n + 108_000_000n,
		})
	})

	it("keeps the fees when the bundler gives neither", async () => {
		stubBundler(async () => ({ jsonrpc: "2.0", id: 1, error: { code: -32601, message: "Method not found" } }))

		expect(await rundlerUserOperationFees("https://other-c.example", fees, params)).toEqual(fees)
	})
})
