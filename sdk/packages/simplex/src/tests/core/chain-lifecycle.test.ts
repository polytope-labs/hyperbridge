import { describe, expect, it, vi } from "vitest"
import { FillerConfigService } from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ConfirmationPolicy } from "@/config/interpolated-curve"
import { EventMonitor } from "@/core/event-monitor"
import type { OrderScanner, OrderScannerHandlers } from "@/scanner/types"
import type { HexString } from "@hyperbridge/sdk"
import { ChainController } from "@/simplex"
import type { FillerRuntime } from "@/core/boot"

// `add` probes the endpoints for real; these are not endpoints.
vi.mock("@/services/FillerConfigService", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/FillerConfigService")>()),
	resolveChainConfigs: async (entries: { rpcUrls: string[]; bundlerUrl?: string }[]) =>
		entries.map((entry) => ({ chainId: 8453, rpcUrls: entry.rpcUrls, bundlerUrl: entry.bundlerUrl })),
}))

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
	/** A monitor wired to a stub scanner, so nothing touches the network. */
	function monitorFor(chainIds: number[], scannerChains = chainIds) {
		const service = configService()
		for (const chainId of chainIds.slice(1)) {
			service.addChain({ chainId, rpcUrls: [`https://rpc-${chainId}.example`] })
		}

		let handlers: OrderScannerHandlers | undefined
		let closed = false
		const scanner: OrderScanner = {
			subscribe: (h) => {
				handlers = h
				return { close: () => (closed = true), dropped: 0 }
			},
			chains: () => [...scannerChains],
			addChain: async () => 0,
			setRpcUrls: async () => {},
			removeChain: async () => {},
			close: async () => {},
		}

		const monitor = new EventMonitor(
			chainIds.map((chainId) => ({ chainId }) as never),
			service,
			{} as unknown as ChainClientManager,
			FILLER,
			scanner,
		)
		return { monitor, service, scanner, handlers: () => handlers, wasClosed: () => closed }
	}

	it("takes one subscription for every chain it watches", async () => {
		const { monitor, handlers } = monitorFor([1, 8453])
		expect(handlers()).toBeUndefined()
		await monitor.startListening()
		expect(handlers()).toBeDefined()
	})

	it("ignores events for chains it is not configured for", async () => {
		// A shared scanner may carry more chains than this filler wants.
		const { monitor, handlers } = monitorFor([1], [1, 8453])
		await monitor.startListening()

		const seen: string[] = []
		monitor.on("newOrder", ({ order }) => seen.push(order.id))

		handlers()!.onOrder({ order: { id: "0xmine" }, transactionHash: "0x1", chainId: 1 } as never)
		handlers()!.onOrder({ order: { id: "0xother" }, transactionHash: "0x2", chainId: 8453 } as never)

		expect(seen).toEqual(["0xmine"])
	})

	it("drops a replayed order", async () => {
		const { monitor, handlers } = monitorFor([1])
		await monitor.startListening()

		const seen: string[] = []
		monitor.on("newOrder", ({ order }) => seen.push(order.id))

		// At-least-once: a scanner resuming from a cursor re-delivers, and the fill
		// path has no idempotency of its own.
		const event = { order: { id: "0xorder" }, transactionHash: "0xtx", chainId: 1 }
		handlers()!.onOrder(event as never)
		handlers()!.onOrder(event as never)

		expect(seen).toEqual(["0xorder"])
	})

	it("only re-emits fills credited to this filler", async () => {
		const { monitor, handlers } = monitorFor([1])
		await monitor.startListening()

		const seen: string[] = []
		monitor.on("orderFilledOnChain", ({ commitment }) => seen.push(commitment))

		// `filler` is indexed:false in the ABI, so every subscriber receives every
		// fill and matches its own address.
		handlers()!.onFill({ commitment: "0xmine", filler: FILLER.toLowerCase(), chainId: 1 } as never)
		handlers()!.onFill({ commitment: "0xtheirs", filler: "0xBBBB", chainId: 1 } as never)

		expect(seen).toEqual(["0xmine"])
	})

	it("starts matching a chain added after boot, once the scanner carries it", async () => {
		const { monitor, handlers } = monitorFor([1], [1, 8453])
		await monitor.startListening()
		await monitor.addChain(8453)

		const seen: string[] = []
		monitor.on("newOrder", ({ order }) => seen.push(order.id))
		handlers()!.onOrder({ order: { id: "0xnew" }, transactionHash: "0x1", chainId: 8453 } as never)

		expect(seen).toEqual(["0xnew"])
	})

	it("refuses a chain the scanner does not carry", async () => {
		const { monitor } = monitorFor([1], [1])
		await monitor.startListening()
		await expect(monitor.addChain(8453)).rejects.toThrow(/not in the order scanner/)
	})

	it("refuses to add a chain it already monitors", async () => {
		const { monitor } = monitorFor([1])
		await expect(monitor.addChain(1)).rejects.toThrow(/already monitored/)
	})

	it("stops matching a removed chain but leaves the scanner alone", async () => {
		const { monitor, handlers } = monitorFor([1, 8453])
		await monitor.startListening()
		await monitor.removeChain(8453)

		const seen: string[] = []
		monitor.on("newOrder", ({ order }) => seen.push(order.id))
		handlers()!.onOrder({ order: { id: "0xgone" }, transactionHash: "0x1", chainId: 8453 } as never)

		expect(seen).toEqual([])
	})

	it("releases its subscription on stop", async () => {
		const { monitor, wasClosed } = monitorFor([1])
		await monitor.startListening()
		await monitor.stopListening()
		expect(wasClosed()).toBe(true)
	})
})

describe("ChainController.add rollback", () => {
	/**
	 * Reported in review on #1123: the supplied-scanner refusal sat below
	 * `configService.addChain`, so a rejected add left the chain registered but
	 * unscanned — absent from `list()`, "already configured" on retry, and picked
	 * up by the next `syncWatchOnlyToConfig()` into the persisted config.
	 */
	it("mutates nothing when it refuses a chain missing from a supplied scanner", async () => {
		const service = configService()
		const before = [...service.getConfiguredChainIds()]

		const intentFiller = {
			setWatchOnly: vi.fn(),
			clearWatchOnly: vi.fn(),
			addChain: vi.fn(),
			getWatchOnly: () => ({}),
		}
		const runtime = {
			configService: service,
			config: { chains: [], confirmationPolicies: {} },
			intentFiller,
			globalWatchOnly: true,
			resolvedChains: [],
			chainClientManager: { invalidate: vi.fn() },
		} as unknown as FillerRuntime

		// A scanner the caller owns that does not carry the chain being added.
		const supplied: OrderScanner = {
			subscribe: () => ({ close: () => {}, dropped: 0 }),
			chains: () => [],
			addChain: vi.fn(async () => 0),
			setRpcUrls: async () => {},
			removeChain: async () => {},
			close: async () => {},
		}
		const controller = new ChainController(runtime, async () => {}, supplied, false)

		await expect(
			controller.add({ rpcUrls: ["https://base.example"], bundlerUrl: "https://bundler.example" }),
		).rejects.toThrow(/not in the order scanner/)

		expect(service.getConfiguredChainIds()).toEqual(before)
		expect(intentFiller.setWatchOnly).not.toHaveBeenCalled()
		expect(supplied.addChain).not.toHaveBeenCalled()
	})
})
