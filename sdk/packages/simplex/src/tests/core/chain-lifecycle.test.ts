import { describe, expect, it, vi } from "vitest"
import { FillerConfigService } from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ConfirmationPolicy } from "@/config/interpolated-curve"
import { EventMonitor } from "@/core/event-monitor"
import type { OrderSource, OrderSourceHandlers } from "@/scanner/types"
import type { HexString } from "@hyperbridge/sdk"

/**
 * The plumbing behind runtime chain edits. Each piece is covered here rather
 * than through `Simplex.chains`, which needs live RPCs to resolve a chain id.
 */

const RPC_A = ["https://eth-a.example"]
const RPC_B = ["https://eth-b.example"]
const FILLER = "0xAAAA00000000000000000000000000000000AAAA" as HexString

function configService(chainId = 1, rpcUrls = RPC_A) {
	return new FillerConfigService([{ chainId, rpcUrls, bundlerUrl: "https://bundler.example" }])
}

describe("FillerConfigService chain set", () => {
	it("derives the configured chain set from the registered chains", () => {
		const service = configService()
		expect(service.getConfiguredChainIds()).toEqual([1])

		service.addChain({ chainId: 8453, rpcUrls: RPC_B, bundlerUrl: "https://bundler-base.example" })
		expect(service.getConfiguredChainIds()).toEqual([1, 8453])
		expect(service.getRpcUrls("EVM-8453")).toEqual(RPC_B)

		service.removeChain(8453)
		expect(service.getConfiguredChainIds()).toEqual([1])
	})

	it("refuses to re-add a configured chain rather than silently repointing it", () => {
		const service = configService()
		expect(() => service.addChain({ chainId: 1, rpcUrls: RPC_B })).toThrow(/already configured/)
		// The original endpoints survive the rejected call.
		expect(service.getRpcUrls("EVM-1")).toEqual(RPC_A)
	})

	it("swaps RPC endpoints on a configured chain", () => {
		const service = configService()
		service.setRpcUrls(1, RPC_B)
		expect(service.getRpcUrls("EVM-1")).toEqual(RPC_B)
	})

	it("rejects endpoint edits for chains it does not know", () => {
		const service = configService()
		expect(() => service.setRpcUrls(999, RPC_B)).toThrow(/not configured/)
		expect(() => service.setBundlerUrl(999, "https://x.example")).toThrow(/not configured/)
	})

	it("rejects duplicate hosts, on add as well as at construction", () => {
		const service = configService()
		expect(() =>
			service.addChain({ chainId: 8453, rpcUrls: ["https://same.example", "https://same.example/2"] }),
		).toThrow()
		expect(service.getConfiguredChainIds()).toEqual([1])
	})
})

describe("ChainClientManager", () => {
	it("does not share clients between instances", () => {
		// The factory used to be a module-level singleton, so two fillers in one
		// process served each other's requests from the same cached client.
		const a = new ChainClientManager(configService(1, RPC_A))
		const b = new ChainClientManager(configService(1, RPC_B))
		expect(a.getPublicClient("EVM-1")).not.toBe(b.getPublicClient("EVM-1"))
	})

	it("caches per chain until invalidated", () => {
		const manager = new ChainClientManager(configService())
		const first = manager.getPublicClient("EVM-1")
		expect(manager.getPublicClient("EVM-1")).toBe(first)

		manager.invalidate("EVM-1")
		expect(manager.getPublicClient("EVM-1")).not.toBe(first)
	})

	it("rebuilds the quorum client on invalidation too", () => {
		const manager = new ChainClientManager(configService())
		const first = manager.getQuorumClient("EVM-1")
		expect(manager.getQuorumClient("EVM-1")).toBe(first)

		manager.invalidate("EVM-1")
		expect(manager.getQuorumClient("EVM-1")).not.toBe(first)
	})

	it("invalidates a chain that is already gone from the config", () => {
		// Removal drops the chain from the config service first, so invalidation
		// must not depend on it still being registered.
		const service = configService()
		const manager = new ChainClientManager(service)
		manager.getPublicClient("EVM-1")
		service.removeChain(1)
		expect(() => manager.invalidate("EVM-1")).not.toThrow()
	})
})

describe("ConfirmationPolicy runtime coverage", () => {
	const curve = { points: [{ amount: "1000", value: 2 }, { amount: "100000", value: 9 }] }

	it("installs a curve for a chain added after boot", () => {
		const policy = new ConfirmationPolicy({ "1": curve })
		expect(policy.has(8453)).toBe(false)
		expect(() => policy.assertCovers([1, 8453])).toThrow()

		policy.add(8453, curve)
		expect(policy.has(8453)).toBe(true)
		expect(() => policy.assertCovers([1, 8453])).not.toThrow()
	})

	it("validates before mutating, so a bad curve leaves coverage untouched", () => {
		const policy = new ConfirmationPolicy({ "1": curve })
		expect(() => policy.add(8453, { points: [{ amount: "1000", value: 2 }] })).toThrow(/at least 2 points/)
		expect(policy.has(8453)).toBe(false)
	})

	it("drops a removed chain's curve", () => {
		const policy = new ConfirmationPolicy({ "1": curve, "8453": curve })
		policy.remove(8453)
		expect(policy.has(8453)).toBe(false)
	})
})

describe("EventMonitor chain lifecycle", () => {
	/** A monitor wired to a stub source, so nothing touches the network. */
	function monitorFor(chainIds: number[]) {
		const service = configService()
		for (const chainId of chainIds.slice(1)) {
			service.addChain({ chainId, rpcUrls: [`https://rpc-${chainId}.example`] })
		}

		const subscribed: number[] = []
		const closed: number[] = []
		const handlers = new Map<number, OrderSourceHandlers>()
		const source: OrderSource = {
			subscribe: (target, h) => {
				subscribed.push(target.chainId)
				handlers.set(target.chainId, h)
				return {
					close: () => closed.push(target.chainId),
					dropped: 0,
				}
			},
			activeChains: () => [...new Set(subscribed)],
		}

		const monitor = new EventMonitor(
			chainIds.map((chainId) => ({ chainId }) as never),
			service,
			{} as unknown as ChainClientManager,
			FILLER,
			source,
		)
		return { monitor, service, source, subscribed, closed, handlers }
	}

	it("subscribes to every constructed chain once listening", async () => {
		const { monitor, subscribed } = monitorFor([1, 8453])
		expect(subscribed).toEqual([])
		await monitor.startListening()
		expect(subscribed.sort()).toEqual([1, 8453])
	})

	it("subscribes a chain added after boot, and not before listening", async () => {
		const { monitor, subscribed } = monitorFor([1])
		await monitor.addChain(8453)
		// Not listening yet, so nothing was subscribed.
		expect(subscribed).toEqual([])

		await monitor.startListening()
		expect(subscribed.sort()).toEqual([1, 8453])
	})

	it("refuses to add a chain it already monitors", async () => {
		const { monitor } = monitorFor([1])
		await expect(monitor.addChain(1)).rejects.toThrow(/already monitored/)
	})

	it("releases a removed chain's subscription and leaves the others alone", async () => {
		const { monitor, closed, subscribed } = monitorFor([1, 8453])
		await monitor.startListening()

		await monitor.removeChain(8453)
		expect(closed).toEqual([8453])
		// The surviving chain is untouched — and the shared loop it holds keeps running.
		expect(subscribed).toContain(1)
	})

	it("resubscribes on rebuild so the filler moves to the loop for its new endpoints", async () => {
		const { monitor, service, closed, subscribed } = monitorFor([1])
		await monitor.startListening()
		expect(subscribed).toEqual([1])

		service.setRpcUrls(1, ["https://new.example"])
		await monitor.rebuildChain(1)

		expect(closed).toEqual([1])
		expect(subscribed).toEqual([1, 1])
	})

	it("refuses to rebuild a chain it does not monitor", async () => {
		const { monitor } = monitorFor([1])
		await expect(monitor.rebuildChain(999)).rejects.toThrow(/not monitored/)
	})

	it("re-emits orders from the source, and drops a replayed one", async () => {
		const { monitor, handlers } = monitorFor([1])
		await monitor.startListening()

		const seen: string[] = []
		monitor.on("newOrder", ({ order }) => seen.push(order.id))

		const event = {
			order: { id: "0xorder" },
			transactionHash: "0xtx",
			blockNumber: 1n,
			blockHash: "0xblock",
			logIndex: 0,
			chain: "EVM-1",
			chainId: 1,
		}
		// At-least-once: a shared feed resuming from a cursor re-delivers, and the fill
		// path has no idempotency of its own.
		handlers.get(1)!.onOrder(event as never)
		handlers.get(1)!.onOrder(event as never)

		expect(seen).toEqual(["0xorder"])
	})

	it("only re-emits fills credited to this filler", async () => {
		const { monitor, handlers } = monitorFor([1])
		await monitor.startListening()

		const seen: string[] = []
		monitor.on("orderFilledOnChain", ({ commitment }) => seen.push(commitment))

		// `filler` is indexed:false in the ABI, so every consumer receives every fill
		// and narrows it locally.
		handlers.get(1)!.onFill({ commitment: "0xmine", filler: FILLER.toLowerCase(), chainId: 1 } as never)
		handlers.get(1)!.onFill({ commitment: "0xtheirs", filler: "0xBBBB", chainId: 1 } as never)

		expect(seen).toEqual(["0xmine"])
	})
})
