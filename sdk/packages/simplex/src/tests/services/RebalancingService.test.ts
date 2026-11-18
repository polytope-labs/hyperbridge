import { afterEach, describe, expect, it, vi } from "vitest"
import { parseUnits } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { AssetRegistry, type AssetDefinition } from "@/config/asset-registry"
import type { RebalancingHubToml } from "@/config/filler-toml"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { FillerConfigService } from "@/services/FillerConfigService"
import {
	RebalancingService,
	type RebalanceTransfer,
	type RebalancingServiceOptions,
} from "@/services/RebalancingService"
import { BinanceRebalancer, CctpRebalancer, Usdt0Rebalancer, type RouteTransfer } from "@/services/rebalancers"

const ETHEREUM = "EVM-1"
const BSC = "EVM-56"
const UNICHAIN = "EVM-130"
const POLYGON = "EVM-137"
const BASE = "EVM-8453"
const ARBITRUM = "EVM-42161"

const WALLET = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const USDC_HUB: RebalancingHubToml = { asset: "USDC", chain: BASE, threshold: "6000", minBalance: "5000" }
const USDT_HUB: RebalancingHubToml = { asset: "USDT", chain: ARBITRUM, threshold: "6000", minBalance: "5000" }
const BINANCE = { apiKey: "key", apiSecret: "secret" }

/** What the solver wallet holds, as `wallets[chain][symbol]` in human units. */
type Wallets = Record<string, Record<string, string | Error>>

/**
 * The real service over the real routes, the SDK chain registry and a wallet that exists only
 * in this test. Nothing reaches a network: balances come from `wallets`, and every route's
 * `send` is replaced by the case.
 */
function setUp(
	options: {
		chains?: string[]
		hubs?: RebalancingHubToml[]
		wallets?: Wallets
		assets?: Record<string, AssetDefinition>
	} & RebalancingServiceOptions = {},
) {
	const chains = options.chains ?? [ETHEREUM, BSC, POLYGON, BASE, ARBITRUM]
	const configService = new FillerConfigService(
		chains.map((chain) => ({ chainId: Number(chain.slice(4)), rpcUrls: [`https://${chain}.rpc.test`] })),
		{ maxConcurrentOrders: 5, rebalancing: { hubs: options.hubs ?? [] } },
	)
	const registry = new AssetRegistry(configService, options.assets)
	// The SDK registry gives BNB Chain's stablecoins 18 decimals and everything else 6.
	const decimalsOn = (chain: string) => (chain === BSC ? 18 : 6)

	const getPublicClient = (chain: string) => ({
		readContract: async ({ address, functionName }: { address: string; functionName: string }) => {
			if (functionName === "decimals") return decimalsOn(chain)
			const symbol = registry.symbolFor(address, chain)
			const held = (symbol && options.wallets?.[chain]?.[symbol]) ?? "0"
			if (held instanceof Error) throw held
			return parseUnits(held, decimalsOn(chain))
		},
	})
	const chainClientManager = {
		loggers: configService.loggers,
		getSigner: () => ({ address: WALLET }),
		getAccount: () => ({ address: WALLET }),
		getPublicClient,
	} as unknown as ChainClientManager

	const service = new RebalancingService(chainClientManager, configService, registry, {
		binance: options.binance,
		hasVault: options.hasVault,
	})
	return { service, configService }
}

/** Replaces every route's `send`, so a case decides how each transfer ends. */
function stubSends(send: (transfer: RouteTransfer) => Promise<{ txHash?: string }>) {
	const stub = vi.fn(send)
	for (const route of [CctpRebalancer, Usdt0Rebalancer, BinanceRebalancer]) {
		vi.spyOn(route.prototype, "send").mockImplementation(stub)
	}
	return stub
}

function deferred<T>() {
	let resolve!: (value: T) => void
	let reject!: (error: unknown) => void
	const promise = new Promise<T>((res, rej) => {
		resolve = res
		reject = rej
	})
	return { promise, resolve, reject }
}

const flush = () => new Promise((resolve) => setImmediate(resolve))

/** A `send` whose calls stay pending until the case settles them, keyed by source chain and asset. */
function pendingSends() {
	const pending = new Map<string, ReturnType<typeof deferred<{ txHash?: string }>>>()
	const send = stubSends(({ asset, source }) => {
		const entry = deferred<{ txHash?: string }>()
		pending.set(`${asset}:${source}`, entry)
		return entry.promise
	})
	const started = () => send.mock.calls.map(([transfer]) => `${transfer.asset}:${transfer.source}`)
	return { pending, started }
}

const transfer = (
	asset: string,
	source: string,
	hub: string,
	method: RebalanceTransfer["method"],
): RebalanceTransfer => ({
	asset,
	source,
	hub,
	amount: "1",
	method,
})

afterEach(() => {
	vi.restoreAllMocks()
})

describe("coverage", () => {
	it("covers the chains CCTP reaches and leaves out the one it does not", () => {
		const { service } = setUp()
		expect(service.coverage(USDC_HUB)).toEqual([
			{ chain: ETHEREUM, method: "cctp" },
			{ chain: BSC, uncovered: "no-route" },
			{ chain: POLYGON, method: "cctp" },
			{ chain: ARBITRUM, method: "cctp" },
		])
	})

	it("uses Binance only where it is configured and no native route runs", () => {
		const { service } = setUp({ binance: BINANCE })
		expect(service.coverage(USDC_HUB)).toEqual([
			{ chain: ETHEREUM, method: "cctp" },
			{ chain: BSC, method: "cex" },
			{ chain: POLYGON, method: "cctp" },
			{ chain: ARBITRUM, method: "cctp" },
		])
	})

	it("does not treat Base as a USDT0 chain: it has a LayerZero id and no USDT0", () => {
		const { service } = setUp()
		expect(service.coverage(USDT_HUB)).toEqual([
			{ chain: ETHEREUM, method: "usdt0" },
			{ chain: BSC, uncovered: "no-route" },
			{ chain: POLYGON, method: "usdt0" },
			{ chain: BASE, uncovered: "no-route" },
		])
	})

	it("leaves out a chain where the asset registry has no such asset", () => {
		const { service } = setUp({ chains: [BASE, UNICHAIN] })
		expect(service.coverage(USDC_HUB)).toEqual([{ chain: UNICHAIN, uncovered: "no-asset" }])
	})

	it("leaves a chain with a vault for the asset to its sweep, but not when the vault is on the hub", () => {
		const hasVault = vi.fn((chain: string) => chain === POLYGON || chain === BASE)
		const { service } = setUp({ chains: [POLYGON, BASE, ARBITRUM], hasVault })
		expect(service.coverage(USDC_HUB)).toEqual([
			{ chain: POLYGON, uncovered: "vault" },
			{ chain: ARBITRUM, method: "cctp" },
		])
		expect(hasVault.mock.calls.map(([chain]) => chain)).not.toContain(BASE)
	})

	it("leaves out a chain whose registry token is not the one the route moves", () => {
		// Bridged USDC.e on Polygon: CCTP would burn native USDC, which the wallet does not hold.
		const assets = { USDC: { [POLYGON]: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174" as HexString } }
		const { service } = setUp({ chains: [POLYGON, BASE, ARBITRUM], assets })
		expect(service.coverage(USDC_HUB)).toEqual([
			{ chain: POLYGON, uncovered: "no-route" },
			{ chain: ARBITRUM, method: "cctp" },
		])
	})
})

describe("catalog", () => {
	it("offers each asset the chains that hold it, with what each would cover as the hub", () => {
		const { service } = setUp({ chains: [BASE, ARBITRUM, UNICHAIN] })
		expect(service.catalog()).toEqual([
			{
				asset: "USDC",
				hubs: [
					{
						chain: BASE,
						chains: [
							{ chain: ARBITRUM, method: "cctp" },
							{ chain: UNICHAIN, uncovered: "no-asset" },
						],
					},
					{
						chain: ARBITRUM,
						chains: [
							{ chain: BASE, method: "cctp" },
							{ chain: UNICHAIN, uncovered: "no-asset" },
						],
					},
				],
			},
			{
				asset: "USDT",
				hubs: [
					{
						chain: BASE,
						chains: [
							{ chain: ARBITRUM, uncovered: "no-route" },
							{ chain: UNICHAIN, uncovered: "no-asset" },
						],
					},
					{
						chain: ARBITRUM,
						chains: [
							{ chain: BASE, uncovered: "no-route" },
							{ chain: UNICHAIN, uncovered: "no-asset" },
						],
					},
				],
			},
		])
	})
})

describe("preflight", () => {
	it("returns what each hub covers", () => {
		const { service } = setUp({ chains: [POLYGON, BASE] })
		expect(service.preflight({ hubs: [USDC_HUB] })).toEqual([
			{ asset: "USDC", hub: BASE, chains: [{ chain: POLYGON, method: "cctp" }] },
		])
	})

	it("rejects a hub on a chain the solver does not run", () => {
		const { service } = setUp({ chains: [POLYGON, ARBITRUM] })
		expect(() => service.preflight({ hubs: [USDC_HUB] })).toThrow(
			/USDC's hub EVM-8453 is not a chain this solver runs/,
		)
	})

	it("rejects an asset no route carries", () => {
		const { service } = setUp()
		expect(() => service.preflight({ hubs: [{ ...USDC_HUB, asset: "DAI" }] })).toThrow(
			/no bridge route carries DAI; routes exist for USDC and USDT/,
		)
	})

	it("rejects a hub chain where the registry has no such asset", () => {
		const { service } = setUp({ chains: [BASE, UNICHAIN] })
		expect(() => service.preflight({ hubs: [{ ...USDC_HUB, chain: UNICHAIN }] })).toThrow(
			/USDC is not in the asset registry on EVM-130/,
		)
	})
})

describe("plan", () => {
	it("bridges a wallet at its threshold down to the minimum balance, each chain on its own", async () => {
		const { service } = setUp({
			hubs: [USDC_HUB],
			wallets: {
				[POLYGON]: { USDC: "7500" },
				[ARBITRUM]: { USDC: "5999.99" },
				[ETHEREUM]: { USDC: "6000" },
				[BSC]: { USDC: "90000" },
			},
		})

		const pass = await service.plan()
		expect(pass.started).toEqual([
			{ asset: "USDC", source: ETHEREUM, hub: BASE, amount: "1000", method: "cctp" },
			{ asset: "USDC", source: POLYGON, hub: BASE, amount: "2500", method: "cctp" },
		])
		expect(pass.skipped).toEqual([
			// No route reaches Base from BNB Chain, whatever it holds. The others still move.
			{ asset: "USDC", chain: BSC, hub: BASE, reason: "no-route" },
			{
				asset: "USDC",
				chain: ARBITRUM,
				hub: BASE,
				reason: "below-threshold",
				walletBalance: "5999.99",
				threshold: "6000",
			},
		])
	})

	it("reads the amounts with each chain's own decimals and sends whole cents", async () => {
		const { service } = setUp({
			chains: [BSC, BASE],
			hubs: [USDC_HUB],
			binance: BINANCE,
			// 18 decimals on BNB Chain: a remainder finer than a cent stays in the wallet.
			wallets: { [BSC]: { USDC: "7500.129999999999999999" } },
		})
		expect((await service.plan()).started).toEqual([
			{ asset: "USDC", source: BSC, hub: BASE, amount: "2500.12", method: "cex" },
		])
	})

	it("keeps planning the other chains when one balance cannot be read", async () => {
		const { service } = setUp({
			chains: [POLYGON, BASE, ARBITRUM],
			hubs: [USDC_HUB],
			wallets: { [POLYGON]: { USDC: new Error("rpc down") }, [ARBITRUM]: { USDC: "6500" } },
		})
		const pass = await service.plan()
		expect(pass.started).toEqual([{ asset: "USDC", source: ARBITRUM, hub: BASE, amount: "1500", method: "cctp" }])
		expect(pass.skipped).toEqual([{ asset: "USDC", chain: POLYGON, hub: BASE, reason: "balance-unavailable" }])
	})

	it("plans every hub, and nothing when no hub is configured", async () => {
		const wallets = { [POLYGON]: { USDC: "7000", USDT: "8000" } }
		const both = setUp({ chains: [POLYGON, BASE, ARBITRUM], hubs: [USDC_HUB, USDT_HUB], wallets })
		expect((await both.service.plan()).started).toEqual([
			{ asset: "USDC", source: POLYGON, hub: BASE, amount: "2000", method: "cctp" },
			{ asset: "USDT", source: POLYGON, hub: ARBITRUM, amount: "3000", method: "usdt0" },
		])

		const none = setUp({ chains: [POLYGON, BASE, ARBITRUM], wallets })
		expect(await none.service.plan()).toEqual({ started: [], skipped: [] })
	})

	it("reads hubs saved while it runs", async () => {
		const { service, configService } = setUp({ chains: [POLYGON, BASE], wallets: { [POLYGON]: { USDC: "7500" } } })
		expect((await service.plan()).started).toEqual([])

		configService.setRebalancing({ hubs: [USDC_HUB], intervalMs: 60_000 })
		expect((await service.plan()).started).toHaveLength(1)
		expect(service.intervalMs()).toBe(60_000)
	})
})

describe("execute", () => {
	/** Both hubs, and 7,000 of each asset on every other chain: every transfer is due for 2,000. */
	const funded = () =>
		setUp({
			hubs: [USDC_HUB, USDT_HUB],
			wallets: Object.fromEntries(
				[ETHEREUM, POLYGON, ARBITRUM, BASE].map((chain) => [chain, { USDC: "7000", USDT: "7000" }]),
			),
		})
	const due = (leg: RebalanceTransfer): RebalanceTransfer => ({ ...leg, amount: "2000" })

	it("sends each transfer through its route and reports what moved", async () => {
		const send = stubSends(async ({ source }) => ({ txHash: `0xburn-${source}` }))
		const { service } = funded()
		const legs = [transfer("USDC", POLYGON, BASE, "cctp"), transfer("USDT", ETHEREUM, ARBITRUM, "usdt0")]

		const outcome = await service.execute(legs)
		expect(send.mock.calls.map(([sent]) => sent)).toEqual([
			{ asset: "USDC", amount: "2000", source: POLYGON, destination: BASE },
			{ asset: "USDT", amount: "2000", source: ETHEREUM, destination: ARBITRUM },
		])
		expect(outcome).toEqual({
			moved: [
				{ ...due(legs[0]), txHash: `0xburn-${POLYGON}` },
				{ ...due(legs[1]), txHash: `0xburn-${ETHEREUM}` },
			],
			failed: [],
			notSent: [],
		})
	})

	it("sends what is above the minimum balance when the transfer's turn comes, not what was planned", async () => {
		const send = stubSends(async () => ({}))
		const wallets = { [POLYGON]: { USDC: "7500" } }
		const { service } = setUp({ chains: [POLYGON, BASE], hubs: [USDC_HUB], wallets })
		const { started } = await service.plan()
		expect(started.map((leg) => leg.amount)).toEqual(["2500"])

		// Fills spent 1,300 while the transfer waited behind another: 2,500 would leave 3,700.
		wallets[POLYGON].USDC = "6200"
		const outcome = await service.execute(started)
		expect(send).toHaveBeenCalledWith({ asset: "USDC", amount: "1200", source: POLYGON, destination: BASE })
		expect(outcome.moved).toEqual([{ ...started[0], amount: "1200" }])
	})

	it("does not send a transfer whose wallet fell back under its threshold while it waited", async () => {
		const send = stubSends(async () => ({}))
		const wallets = { [POLYGON]: { USDC: "7500" } }
		const { service } = setUp({ chains: [POLYGON, BASE], hubs: [USDC_HUB], wallets })
		const { started } = await service.plan()

		wallets[POLYGON].USDC = "5800"
		expect(await service.execute(started)).toEqual({
			moved: [],
			failed: [],
			notSent: [{ ...started[0], reason: "below-threshold" }],
		})
		expect(send).not.toHaveBeenCalled()
	})

	it("does not send a transfer whose hub was moved or removed while it waited", async () => {
		const send = stubSends(async () => ({}))
		const { service, configService } = setUp({
			chains: [POLYGON, BASE, ARBITRUM],
			hubs: [USDC_HUB],
			wallets: { [POLYGON]: { USDC: "7500" } },
		})
		const { started } = await service.plan()

		configService.setRebalancing({ hubs: [{ ...USDC_HUB, chain: ARBITRUM }] })
		expect((await service.execute(started)).notSent).toEqual([{ ...started[0], reason: "hub-changed" }])
		configService.setRebalancing(undefined)
		expect((await service.execute(started)).notSent).toEqual([{ ...started[0], reason: "hub-changed" }])
		expect(send).not.toHaveBeenCalled()
	})

	it("runs CCTP transfers into one hub one after another, since each mints there from the solver wallet", async () => {
		const { pending, started } = pendingSends()
		const { service } = funded()

		const run = service.execute([
			transfer("USDC", POLYGON, BASE, "cctp"),
			transfer("USDC", ARBITRUM, BASE, "cctp"),
			// USDT0 is delivered by LayerZero: nothing is sent from the solver wallet on the hub.
			transfer("USDT", ETHEREUM, ARBITRUM, "usdt0"),
		])
		await flush()
		expect(started()).toEqual([`USDC:${POLYGON}`, `USDT:${ETHEREUM}`])

		pending.get(`USDC:${POLYGON}`)!.resolve({})
		await flush()
		expect(started()).toEqual([`USDC:${POLYGON}`, `USDT:${ETHEREUM}`, `USDC:${ARBITRUM}`])

		pending.get(`USDC:${ARBITRUM}`)!.resolve({})
		pending.get(`USDT:${ETHEREUM}`)!.resolve({})
		expect((await run).failed).toEqual([])
	})

	it("runs two transfers out of one chain one after another, and other chains in parallel", async () => {
		const { pending, started } = pendingSends()
		const { service } = funded()

		const run = service.execute([
			transfer("USDT", POLYGON, ARBITRUM, "usdt0"),
			transfer("USDC", POLYGON, BASE, "cctp"),
			transfer("USDT", ETHEREUM, ARBITRUM, "usdt0"),
		])
		await flush()
		expect(started()).toEqual([`USDT:${POLYGON}`, `USDT:${ETHEREUM}`])

		pending.get(`USDT:${POLYGON}`)!.resolve({})
		await flush()
		expect(started()).toEqual([`USDT:${POLYGON}`, `USDT:${ETHEREUM}`, `USDC:${POLYGON}`])

		pending.get(`USDC:${POLYGON}`)!.resolve({})
		pending.get(`USDT:${ETHEREUM}`)!.resolve({})
		expect((await run).moved).toHaveLength(3)
	})

	it("does not start the transfers still waiting once it is cancelled, and finishes the one under way", async () => {
		const { pending, started } = pendingSends()
		const { service } = funded()
		let stopping = false
		const legs = [transfer("USDC", POLYGON, BASE, "cctp"), transfer("USDC", ARBITRUM, BASE, "cctp")]

		const run = service.execute(legs, () => stopping)
		await flush()
		stopping = true
		pending.get(`USDC:${POLYGON}`)!.resolve({})

		expect(await run).toEqual({
			moved: [due(legs[0])],
			failed: [],
			notSent: [{ ...legs[1], reason: "cancelled" }],
		})
		expect(started()).toEqual([`USDC:${POLYGON}`])
	})

	it("reports a failed transfer and still sends the one waiting behind it", async () => {
		const send = stubSends(async ({ source }) => {
			if (source === POLYGON) throw new Error("bridge down")
			return {}
		})
		const { service } = funded()
		const legs = [transfer("USDC", POLYGON, BASE, "cctp"), transfer("USDC", ARBITRUM, BASE, "cctp")]

		const outcome = await service.execute(legs)
		expect(send).toHaveBeenCalledTimes(2)
		expect(outcome).toEqual({
			moved: [due(legs[1])],
			failed: [{ ...due(legs[0]), error: "bridge down" }],
			notSent: [],
		})
	})

	it("reports a transfer whose route is not running, rather than sending it another way", async () => {
		const send = stubSends(async () => ({}))
		// No Binance credentials, so the route a `cex` transfer names does not exist.
		const { service } = funded()
		const leg = transfer("USDC", BSC, BASE, "cex")

		expect(await service.execute([leg])).toEqual({
			moved: [],
			failed: [{ ...leg, error: "The cex route is not available" }],
			notSent: [],
		})
		expect(send).not.toHaveBeenCalled()
	})
})

describe("CctpRebalancer.send", () => {
	it("treats a transfer Bridge Kit did not finish as failed, naming the step and the burn", async () => {
		const { service } = setUp({ hubs: [USDC_HUB], wallets: { [POLYGON]: { USDC: "7000" } } })
		vi.spyOn(CctpRebalancer.prototype, "sendCctp").mockResolvedValue({
			state: "error",
			steps: [
				// The approval comes first and has a hash too; it is not what left the chain.
				{ name: "approve", state: "success", txHash: "0xapprove" },
				{ name: "burn", state: "success", txHash: "0xburn" },
				{ name: "mint", state: "error", errorMessage: "attestation timed out" },
			],
		} as never)

		const leg = { ...transfer("USDC", POLYGON, BASE, "cctp"), amount: "2000" }
		expect(await service.execute([leg])).toEqual({
			moved: [],
			failed: [{ ...leg, error: "CCTP transfer ended error at mint: attestation timed out (source tx 0xburn)" }],
			notSent: [],
		})
	})
})
