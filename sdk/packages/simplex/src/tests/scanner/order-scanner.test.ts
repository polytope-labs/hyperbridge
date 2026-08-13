import { describe, expect, it, vi } from "vitest"
import { OrderScanner } from "@/scanner/order-scanner"
import type { ChainScanner } from "@/scanner/chain-scanner"
import type { ScannedOrder } from "@/scanner/types"

/** What the next endpoint probe answers with. Set per test. */
let chainIdOfEndpoint = 8453

vi.mock("@/services/FillerConfigService", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/FillerConfigService")>()),
	fetchChainId: async () => chainIdOfEndpoint,
}))

/**
 * Sharing is explicit: you build a scanner, hand it to the fillers that should
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
		const scanner = await OrderScanner.create(CHAINS)
		expect(scanner.chains().sort()).toEqual([8453, 42161].sort())
		await scanner.close()
	})

	it("rejects two entries for the same chain", async () => {
		await expect(OrderScanner.create([CHAINS[0], { ...CHAINS[0] }])).rejects.toThrow(/already in this scanner/)
	})

	it("delivers one scanned order to every subscriber", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		const a: string[] = []
		const b: string[] = []
		scanner.subscribe({ onOrder: (e) => a.push(e.order.id!), onFill: () => {} })
		scanner.subscribe({ onOrder: (e) => b.push(e.order.id!), onFill: () => {} })

		// biome-ignore lint/suspicious/noExplicitAny: injecting one event into the live scanner
		;(scanner as any).orders.publish(orderOn(8453, "0xorder"))
		await settle()

		// This is the whole point: two fillers, one scan.
		expect(a).toEqual(["0xorder"])
		expect(b).toEqual(["0xorder"])
		await scanner.close()
	})

	it("stops delivering to a closed subscriber but keeps going for the rest", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		const a: string[] = []
		const b: string[] = []
		const first = scanner.subscribe({ onOrder: (e) => a.push(e.order.id!), onFill: () => {} })
		scanner.subscribe({ onOrder: (e) => b.push(e.order.id!), onFill: () => {} })

		first.close()
		// biome-ignore lint/suspicious/noExplicitAny: injecting one event into the live scanner
		;(scanner as any).orders.publish(orderOn(8453, "0xorder"))
		await settle()

		expect(a).toEqual([])
		expect(b).toEqual(["0xorder"])
		await scanner.close()
	})

	it("carries chains added and removed at runtime", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		await scanner.addChain(CHAINS[1])
		expect(scanner.chains().sort()).toEqual([8453, 42161].sort())

		await scanner.removeChain(42161)
		expect(scanner.chains()).toEqual([8453])
		await scanner.close()
	})

	/**
	 * The reason `setRpcUrls` exists rather than remove-then-add: a new `ChainScanner`
	 * starts from the head, so rebuilding one silently skips every block since the last
	 * scan. With one cursor shared by every filler, that gap is missed by all of them.
	 */
	it("keeps the block cursor when endpoints are swapped", async () => {
		chainIdOfEndpoint = 8453
		const scanner = await OrderScanner.create([CHAINS[0]])
		const loop = (scanner as unknown as { scanners: Map<number, ChainScanner> }).scanners.get(8453)!
		;(loop as unknown as { cursor: bigint }).cursor = 4_242n

		await scanner.setRpcUrls(8453, ["https://base-new.example"])

		const after = (scanner as unknown as { scanners: Map<number, ChainScanner> }).scanners.get(8453)!
		expect(after).toBe(loop)
		expect(after.scannedTo).toBe(4_242n)
		await scanner.close()
	})

	it("refuses endpoints that answer for a different chain", async () => {
		chainIdOfEndpoint = 8453
		const scanner = await OrderScanner.create([CHAINS[0]])

		chainIdOfEndpoint = 137
		await expect(scanner.setRpcUrls(8453, ["https://polygon.example"])).rejects.toThrow(
			/answer for chain 137/,
		)
		// Unchanged: the probe runs before anything is mutated.
		expect((scanner as unknown as { scanners: Map<number, ChainScanner> }).scanners.get(8453)).toBeDefined()
		await scanner.close()
	})

	it("refuses to repoint a chain it does not have", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		await expect(scanner.setRpcUrls(137, ["https://polygon.example"])).rejects.toThrow(
			/not in this scanner/,
		)
		await scanner.close()
	})

	it("refuses to be used after closing", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		await scanner.close()

		expect(() => scanner.subscribe({ onOrder: () => {}, onFill: () => {} })).toThrow(/closed/)
		await expect(scanner.addChain(CHAINS[1])).rejects.toThrow(/closed/)
		await expect(scanner.setRpcUrls(8453, ["https://base.example"])).rejects.toThrow(/closed/)
	})

	it("is idempotent on close", async () => {
		const scanner = await OrderScanner.create([CHAINS[0]])
		await scanner.close()
		await expect(scanner.close()).resolves.toBeUndefined()
	})
})
