import { afterEach, describe, expect, it, vi } from "vitest"
import { applyRundlerPriorityFee, fetchRundlerPriorityFee } from "@/protocols/intents/rundlerFees"

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
