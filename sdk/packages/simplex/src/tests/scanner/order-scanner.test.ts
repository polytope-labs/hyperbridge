import { describe, expect, it } from "vitest"
import { OrderScanner } from "@/scanner/order-scanner"
import type { ScannedOrder } from "@/scanner/types"

/**
 * Sharing is explicit: you build a stream, hand it to the fillers that should
 * share it, and close it yourself. These cover the ownership rules; the scan
 * loop never runs here because no endpoint answers, which is fine — what matters
 * is who receives what and who is allowed to close it.
 */

const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

const CHAINS = [
	{ rpcUrls: ["https://base.example"], bundlerUrl: "https://bundler/base", chainId: 8453, gateway: "0xAA" as const },
	{ rpcUrls: ["https://arb.example"], bundlerUrl: "https://bundler/arb", chainId: 42161, gateway: "0xBB" as const },
]

function orderOn(chainId: number, id: string): ScannedOrder {
	return {
		order: { id } as never,
		transactionHash: "0xtx",
		blockNumber: 1n,
		blockHash: "0xblock",
		logIndex: 0,
		chain: `EVM-${chainId}`,
		chainId,
	}
}

describe("OrderScanner", () => {
	it("scans exactly the chains it was created with", async () => {
		const stream = await OrderScanner.create(CHAINS)
		expect(stream.chains().sort()).toEqual([8453, 42161].sort())
		await stream.close()
	})

	it("rejects two entries for the same chain", async () => {
		await expect(OrderScanner.create([CHAINS[0], { ...CHAINS[0] }])).rejects.toThrow(/already in this scanner/)
	})

	it("delivers one scanned order to every subscriber", async () => {
		const stream = await OrderScanner.create([CHAINS[0]])
		const a: string[] = []
		const b: string[] = []
		stream.subscribe({ onOrder: (e) => a.push(e.order.id!), onFill: () => {} })
		stream.subscribe({ onOrder: (e) => b.push(e.order.id!), onFill: () => {} })

		// biome-ignore lint/suspicious/noExplicitAny: injecting one event into the live stream
		;(stream as any).orders.publish(orderOn(8453, "0xorder"))
		await settle()

		// This is the whole point: two fillers, one scan.
		expect(a).toEqual(["0xorder"])
		expect(b).toEqual(["0xorder"])
		await stream.close()
	})

	it("stops delivering to a closed subscriber but keeps going for the rest", async () => {
		const stream = await OrderScanner.create([CHAINS[0]])
		const a: string[] = []
		const b: string[] = []
		const first = stream.subscribe({ onOrder: (e) => a.push(e.order.id!), onFill: () => {} })
		stream.subscribe({ onOrder: (e) => b.push(e.order.id!), onFill: () => {} })

		first.close()
		// biome-ignore lint/suspicious/noExplicitAny: injecting one event into the live stream
		;(stream as any).orders.publish(orderOn(8453, "0xorder"))
		await settle()

		expect(a).toEqual([])
		expect(b).toEqual(["0xorder"])
		await stream.close()
	})

	it("carries chains added and removed at runtime", async () => {
		const stream = await OrderScanner.create([CHAINS[0]])
		await stream.addChain(CHAINS[1])
		expect(stream.chains().sort()).toEqual([8453, 42161].sort())

		await stream.removeChain(42161)
		expect(stream.chains()).toEqual([8453])
		await stream.close()
	})

	it("refuses to be used after closing", async () => {
		const stream = await OrderScanner.create([CHAINS[0]])
		await stream.close()

		expect(() => stream.subscribe({ onOrder: () => {}, onFill: () => {} })).toThrow(/closed/)
		await expect(stream.addChain(CHAINS[1])).rejects.toThrow(/closed/)
	})

	it("is idempotent on close", async () => {
		const stream = await OrderScanner.create([CHAINS[0]])
		await stream.close()
		await expect(stream.close()).resolves.toBeUndefined()
	})
})
