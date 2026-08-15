import {
	UiServer,
	type AdminStrategy,
	type HaltControl,
	type OperatorContext,
	type PauseControl,
} from "@/services/server/UiServer"
import type { SetupDeps } from "@/services/server/setup-api"
import { ActivityRecorder } from "@/data/recorder"
import { MemoryDataStore } from "@/data/memory"
import { LoggerContext, type LogLevel } from "@/services/Logger"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import type { FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"
import { describe, it, expect, afterEach, vi } from "vitest"
import { existsSync, mkdtempSync, readFileSync, writeFileSync, mkdirSync } from "fs"
import { createConnection } from "net"
import { tmpdir } from "os"
import { dirname, join } from "path"
import { parse } from "toml"
import Decimal from "decimal.js"

/** fetch() normalizes `..` out of URLs and forbids Host, so these tests need a raw socket. */
function rawRequest(port: number, path: string, host = "127.0.0.1"): Promise<string> {
	return new Promise((resolve, reject) => {
		const socket = createConnection(port, "127.0.0.1", () => {
			socket.write(`GET ${path} HTTP/1.1\r\nHost: ${host}\r\nConnection: close\r\n\r\n`)
		})
		let data = ""
		socket.on("data", (chunk) => {
			data += chunk.toString()
		})
		socket.on("end", () => resolve(data))
		socket.on("error", reject)
	})
}

// Covers the UI server's operator surface: inflight price curve changes (the
// server holds the same policy instances the strategies price with), pause and
// balance endpoints, CSRF header enforcement, and mode gating.

const BID_POINTS = [
	{ amount: "100", price: "1580" },
	{ amount: "5000", price: "1570" },
]
const ASK_POINTS = [
	{ amount: "100", price: "1560" },
	{ amount: "5000", price: "1550" },
]
// Same-token transfer market: ask prices strictly below par.
const SAME_ASSET_POINTS = [
	{ amount: "100", price: "0.99" },
	{ amount: "100000", price: "0.999" },
]

const CSRF = { "X-Simplex-UI": "1" }

function fakePauseControl(): PauseControl & { paused: boolean } {
	return {
		paused: false,
		pause() {
			this.paused = true
		},
		resume() {
			this.paused = false
		},
		isPaused() {
			return this.paused
		},
		getWatchOnly() {
			return { 56: true }
		},
	}
}

function fakeHaltControl(index: number, halted = false): HaltControl & { halted: boolean } {
	return {
		index,
		halted,
		isHalted() {
			return this.halted
		},
		resetHalt() {
			this.halted = false
		},
	}
}

/** pairs[] indices line up with the AdminStrategy pairIndex values used in the tests. */
function fakeConfig(): FillerTomlConfig {
	return {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0xab" },
			maxConcurrentOrders: 5,
			queue: { maxRechecks: 10, recheckDelayMs: 30000 },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example",
		},
		pairs: [
			{ token0: "USDC", token1: "USDC", maxOrderSize: "100000", askPriceCurve: SAME_ASSET_POINTS },
			{
				token0: "USDC",
				token1: "CNGN",
				maxOrderSize: "5000",
				bidPriceCurve: BID_POINTS,
				askPriceCurve: ASK_POINTS,
			},
			// venue-priced: no curves
			{ token0: "USDC", token1: "CNGN", maxOrderSize: "5000" },
			// one-sided LP
			{ token0: "USDC", token1: "ZARP", maxOrderSize: "5000", askPriceCurve: ASK_POINTS },
		],
		chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
	}
}

type TestOperator = OperatorContext & { data: MemoryDataStore; loggers: LoggerContext }

function baseOperator(overrides: Partial<OperatorContext> = {}): TestOperator {
	const dataDir = mkdtempSync(join(tmpdir(), "simplex-ui-"))
	const data = new MemoryDataStore()
	const loggers = new LoggerContext()
	return {
		data,
		loggers,
		strategies: [],
		filler: fakePauseControl(),
		balances: { getSnapshot: () => ({ updatedAt: null, chains: [] }) },
		haltControls: [],
		config: fakeConfig(),
		stop: vi.fn().mockResolvedValue(undefined),
		activity: new ActivityRecorder(data.activity),
		setPaused: (paused: boolean) => data.state.set({ paused }),
		setLogLevel: (level: LogLevel) => loggers.setLevel(level),
		applyAllowlist: vi.fn(),
		applyRebalancing: vi.fn(),
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(dataDir, "filler-config.toml"),
		chains: [8453, 56],
		strategyTypes: ["USDC/CNGN"],
		...overrides,
	}
}

describe("FillerPricePolicy runtime mutation", () => {
	it("getPoints returns points sorted by amount as strings", () => {
		const policy = new FillerPricePolicy({
			points: [
				{ amount: "5000", price: "1570" },
				{ amount: "100", price: "1580" },
			],
		})
		expect(policy.getPoints()).toEqual([
			{ amount: "100", price: "1580" },
			{ amount: "5000", price: "1570" },
		])
	})

	it("replacePoints changes what getPrice returns on the same instance", () => {
		const policy = new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] })
		expect(policy.getPrice(new Decimal(1000)).toString()).toBe("1500")

		policy.replacePoints({
			points: [
				{ amount: "0", price: "1600" },
				{ amount: "2000", price: "1700" },
			],
		})
		expect(policy.getPrice(new Decimal(1000)).toString()).toBe("1650")
	})

	it("replacePoints rejects invalid input and leaves the curve unchanged", () => {
		const policy = new FillerPricePolicy({ points: BID_POINTS })
		expect(() => policy.replacePoints({ points: [{ amount: "0", price: "-5" }] })).toThrow(/positive/)
		expect(() => policy.replacePoints({ points: [] })).toThrow(/at least 1 point/)
		expect(policy.getPoints()).toEqual(BID_POINTS)
	})
})

describe("UiServer (operator mode)", () => {
	let server: UiServer | undefined

	afterEach(() => {
		server?.stop()
		server = undefined
	})

	async function startServer(overrides: Partial<OperatorContext> = {}, deps?: SetupDeps) {
		const sameAsset = new FillerPricePolicy({ points: SAME_ASSET_POINTS })
		const bid = new FillerPricePolicy({ points: BID_POINTS })
		const ask = new FillerPricePolicy({ points: ASK_POINTS })
		const askOnly = new FillerPricePolicy({ points: ASK_POINTS })
		const filler = fakePauseControl()
		const operator = baseOperator({
			strategies: [
				{ index: 0, pairIndex: 0, exotic: "USDC/USDC", token0: "USDC", token1: "USDC", ask: sameAsset, sameToken: true, maxOrderSize: "100000" },
				{ index: 1, pairIndex: 1, exotic: "USDC/CNGN", token0: "USDC", token1: "CNGN", bid, ask, sameToken: false, maxOrderSize: "5000" },
				{ index: 2, pairIndex: 2, token0: "USDC", token1: "CNGN", sameToken: false }, // venue-priced: no editable curves
				{ index: 3, pairIndex: 3, exotic: "USDC/ZARP", token0: "USDC", token1: "ZARP", ask: askOnly, sameToken: false, maxOrderSize: "5000" }, // one-sided LP
			],
			filler,
			balances: { getSnapshot: () => ({ updatedAt: 123, chains: [{ chainId: 8453, usdc: 1500 }] }) },
			...overrides,
		})
		server = new UiServer({ mode: "operator", operator, deps })
		const port = await server.start(0)
		return {
			base: `http://127.0.0.1:${port}`,
			sameAsset,
			bid,
			ask,
			askOnly,
			filler,
			dataDir: dirname(operator.configPath!),
			operator,
		}
	}

	async function put(base: string, path: string, body: unknown, headers: Record<string, string> = CSRF) {
		return fetch(`${base}${path}`, {
			method: "PUT",
			headers: { "Content-Type": "application/json", ...headers },
			body: JSON.stringify(body),
		})
	}

	it("serves health and status", async () => {
		const { base } = await startServer()
		const health = await fetch(`${base}/health`)
		expect(await health.json()).toEqual({ status: "ok", mode: "operator" })

		const status = await fetch(`${base}/api/status`)
		expect(status.status).toBe(200)
		const payload = await status.json()
		expect(payload.mode).toBe("operator")
		expect(payload.paused).toBe(false)
		expect(payload.chains).toEqual([8453, 56])
		expect(payload.watchOnly).toEqual({ "56": true })
		expect(payload.strategyTypes).toEqual(["USDC/CNGN"])

		const balances = await fetch(`${base}/api/balances`)
		expect(await balances.json()).toEqual({ updatedAt: 123, chains: [{ chainId: 8453, usdc: 1500 }] })
	})

	it("rejects mutating requests without the X-Simplex-UI header", async () => {
		const { base, bid } = await startServer()
		const res = await put(base, "/api/strategies/1/curves", { bidPriceCurve: BID_POINTS }, {})
		expect(res.status).toBe(403)
		expect(bid.getPoints()).toEqual(BID_POINTS)

		const pause = await fetch(`${base}/api/pause`, { method: "POST" })
		expect(pause.status).toBe(403)
	})

	it("rejects DNS-name Host headers (DNS rebinding) and serves loopback Hosts", async () => {
		const { base } = await startServer()
		const port = Number(new URL(base).port)
		// A rebound attacker origin presents its own domain in Host.
		expect(await rawRequest(port, "/api/status", "evil.example.com")).toContain("403")
		expect(await rawRequest(port, "/api/status", "evil.example.com:1234")).toContain("403")
		expect(await rawRequest(port, "/health", "evil.example.com")).toContain("403")
		// Legitimate local access keeps working.
		expect(await rawRequest(port, "/api/status", `127.0.0.1:${port}`)).toContain("200")
		expect(await rawRequest(port, "/api/status", "localhost")).toContain("200")
	})

	it("lists strategies with their curves", async () => {
		const { base } = await startServer()
		const res = await fetch(`${base}/api/strategies`)
		expect(res.status).toBe(200)
		expect(await res.json()).toEqual({
			strategies: [
				{
					index: 0,
					exotic: "USDC/USDC",
					token0: "USDC",
					token1: "USDC",
					pricingMode: "static",
					sameToken: true,
					referenceOnly: false,
					maxOrderSize: "100000",
					ask: SAME_ASSET_POINTS,
				},
				{
					index: 1,
					exotic: "USDC/CNGN",
					token0: "USDC",
					token1: "CNGN",
					pricingMode: "static",
					sameToken: false,
					referenceOnly: false,
					maxOrderSize: "5000",
					bid: BID_POINTS,
					ask: ASK_POINTS,
				},
				{ index: 2, token0: "USDC", token1: "CNGN", pricingMode: "venue", sameToken: false, referenceOnly: false },
				{
					index: 3,
					exotic: "USDC/ZARP",
					token0: "USDC",
					token1: "ZARP",
					pricingMode: "static",
					sameToken: false,
					referenceOnly: false,
					maxOrderSize: "5000",
					ask: ASK_POINTS,
				},
			],
		})
	})

	it("applies a curve update to the live policy instance and persists it to the config", async () => {
		const { base, bid, ask, operator } = await startServer()
		const newAsk = [
			{ amount: "0", price: "1540" },
			{ amount: "1000", price: "1535" },
		]
		const res = await put(base, "/api/strategies/1/curves", { askPriceCurve: newAsk })
		expect(res.status).toBe(200)
		expect(await res.json()).toEqual({
			index: 1,
			exotic: "USDC/CNGN",
			token0: "USDC",
			token1: "CNGN",
			pricingMode: "static",
			sameToken: false,
			referenceOnly: false,
			maxOrderSize: "5000",
			bid: BID_POINTS,
			ask: newAsk,
			persisted: true,
		})
		expect(ask.getPoints()).toEqual(newAsk)
		expect(bid.getPoints()).toEqual(BID_POINTS)

		// restarts keep the change: the config file now carries the new curve
		expect(existsSync(operator.configPath!)).toBe(true)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.pairs?.[1]?.askPriceCurve).toEqual(newAsk)
		expect(written.pairs?.[1]?.bidPriceCurve).toEqual(BID_POINTS)
	})

	it("rejects same-token ask prices at or above par, judged against the live invariants", async () => {
		const { base, sameAsset } = await startServer()
		const res = await put(base, "/api/strategies/0/curves", {
			askPriceCurve: [{ amount: "0", price: "1" }],
		})
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("below 1")
		expect(sameAsset.getPoints()).toEqual(SAME_ASSET_POINTS)
	})

	it("rejects enabling a bid on a same-token market", async () => {
		const { base } = await startServer()
		const res = await put(base, "/api/strategies/0/curves", { bidPriceCurve: [{ amount: "0", price: "0.9" }] })
		expect(res.status).toBe(409)
		expect((await res.json()).error).toContain("ask-only")
	})

	it("accepts an edit that crosses the book — sides are quoted independently", async () => {
		const { base, bid, ask } = await startServer()
		// New ask above the existing bid at every size: crossed, but allowed —
		// each side fills at its own curve.
		const res = await put(base, "/api/strategies/1/curves", {
			askPriceCurve: [{ amount: "0", price: "1650" }],
		})
		expect(res.status).toBe(200)
		expect(bid.getPoints()).toEqual(BID_POINTS)
		expect(ask.getPoints()).toEqual([{ amount: "0", price: "1650" }])
	})

	it("rejects malformed bodies with 400", async () => {
		const { base } = await startServer()
		expect((await put(base, "/api/strategies/1/curves", {})).status).toBe(400)
		expect((await put(base, "/api/strategies/1/curves", { bidPriceCurve: "flat" })).status).toBe(400)
		expect((await put(base, "/api/strategies/1/curves", { bidPriceCurve: [{ amount: 5, price: "1" }] })).status).toBe(
			400,
		)
		expect((await put(base, "/api/strategies/1/curves", { unexpected: true })).status).toBe(400)
	})

	it("is all-or-nothing: an invalid ask rejects the whole update including a valid bid", async () => {
		const { base, bid, ask } = await startServer()
		const res = await put(base, "/api/strategies/1/curves", {
			bidPriceCurve: [{ amount: "0", price: "1600" }],
			askPriceCurve: [{ amount: "0", price: "-5" }],
		})
		expect(res.status).toBe(400)
		expect(bid.getPoints()).toEqual(BID_POINTS)
		expect(ask.getPoints()).toEqual(ASK_POINTS)
	})

	it("returns 404 for unknown strategies and routes", async () => {
		const { base } = await startServer()
		expect((await put(base, "/api/strategies/9/curves", { bidPriceCurve: BID_POINTS })).status).toBe(404)
		expect((await fetch(`${base}/api/nope`)).status).toBe(404)
	})

	it("returns 409 for venue-priced strategies and disabled sides", async () => {
		const { base, askOnly } = await startServer()
		expect((await put(base, "/api/strategies/2/curves", { bidPriceCurve: BID_POINTS })).status).toBe(409)
		expect((await put(base, "/api/strategies/3/curves", { bidPriceCurve: BID_POINTS })).status).toBe(409)
		expect(askOnly.getPoints()).toEqual(ASK_POINTS)
	})

	it("returns 405 for wrong methods", async () => {
		const { base } = await startServer()
		expect((await fetch(`${base}/api/strategies`, { method: "PUT", headers: CSRF, body: "{}" })).status).toBe(405)
		expect((await fetch(`${base}/api/strategies/1`, { method: "POST", headers: CSRF, body: "{}" })).status).toBe(405)
	})

	it("pause/resume toggles the filler and persists the state", async () => {
		const { base, filler, operator } = await startServer()

		const pause = await fetch(`${base}/api/pause`, { method: "POST", headers: CSRF })
		expect(await pause.json()).toEqual({ paused: true })
		expect(filler.paused).toBe(true)
		expect(await operator.data.state.get()).toEqual({ paused: true })

		const resume = await fetch(`${base}/api/resume`, { method: "POST", headers: CSRF })
		expect(await resume.json()).toEqual({ paused: false })
		expect(filler.paused).toBe(false)
		expect(await operator.data.state.get()).toEqual({ paused: false })
	})

	it("surfaces halted strategies in status and resets them", async () => {
		const halt = fakeHaltControl(1, true)
		const { base } = await startServer({ haltControls: [halt] })

		const status = await (await fetch(`${base}/api/status`)).json()
		expect(status.halted).toEqual([1])

		const res = await fetch(`${base}/api/reset-halt`, { method: "POST", headers: CSRF })
		expect(await res.json()).toEqual({ halted: [] })
		expect(halt.halted).toBe(false)
		expect((await (await fetch(`${base}/api/status`)).json()).halted).toEqual([])
	})

	it("stop drains the runtime via the operator callback", async () => {
		const { base, operator } = await startServer()
		const res = await fetch(`${base}/api/stop`, { method: "POST", headers: CSRF })
		expect(res.status).toBe(202)
		expect(await res.json()).toEqual({ stopping: true })
		await vi.waitFor(() => expect(operator.stop).toHaveBeenCalledTimes(1))
	})

	it("serves the order activity feed with paging", async () => {
		const { base, operator } = await startServer()
		const activity = operator.data.activity
		await activity.record({ type: "detected", orderId: "order-1" })
		await activity.record({ type: "skipped", orderId: "order-1", reason: "No profitable strategy" })
		await activity.record({ type: "filled", orderId: "order-2", volumeUsd: 120, profitUsd: 1.2, chainId: 8453 })

		const res = await (await fetch(`${base}/api/activity/orders?limit=2`)).json()
		expect(res.events).toHaveLength(2)
		expect(res.events[0].type).toBe("filled")
		expect(res.events[1].reason).toBe("No profitable strategy")

		const older = await (
			await fetch(`${base}/api/activity/orders?limit=10&before=${res.events[1].id}`)
		).json()
		expect(older.events).toHaveLength(1)
		expect(older.events[0].type).toBe("detected")
	})

	it("streams live activity over SSE", async () => {
		const { base, operator } = await startServer()
		const recorder = operator.activity

		const controller = new AbortController()
		const response = await fetch(`${base}/api/events`, { signal: controller.signal })
		expect(response.headers.get("content-type")).toContain("text/event-stream")
		const reader = response.body!.getReader()

		recorder.record({ type: "detected", orderId: "live-order" })

		let received = ""
		while (!received.includes("live-order")) {
			const { value, done } = await reader.read()
			if (done) break
			received += new TextDecoder().decode(value)
		}
		controller.abort()

		const dataLine = received.split("\n").find((line) => line.startsWith("data: "))
		expect(dataLine).toBeDefined()
		expect(JSON.parse(dataLine!.slice(6)).orderId).toBe("live-order")
	})

	it("changes the log level and persists it", async () => {
		const { base, operator } = await startServer()
		const res = await fetch(`${base}/api/log-level`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ level: "warn" }),
		})
		expect(await res.json()).toEqual({ level: "warn", persisted: true })
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.simplex.logging).toBe("warn")

		const bad = await fetch(`${base}/api/log-level`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ level: "loud" }),
		})
		expect(bad.status).toBe(400)
	})

	it("updates the allowlist at runtime and persists it", async () => {
		const { base, operator } = await startServer()
		const user = "0x1111111111111111111111111111111111111111"

		const res = await fetch(`${base}/api/allowlist`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ users: [user] }),
		})
		expect(await res.json()).toEqual({ users: [user], persisted: true })
		expect(operator.applyAllowlist).toHaveBeenCalledWith({ users: [user] })
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.allowlist?.users).toEqual([user])

		// empty list removes the allowlist entirely (accept everyone)
		await fetch(`${base}/api/allowlist`, { method: "PUT", headers: CSRF, body: JSON.stringify({ users: [] }) })
		expect(operator.applyAllowlist).toHaveBeenLastCalledWith(undefined)

		const bad = await fetch(`${base}/api/allowlist`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ users: ["nope"] }),
		})
		expect(bad.status).toBe(400)
	})

	it("exposes vault controls only when a vault is configured", async () => {
		const none = await startServer()
		expect((await fetch(`${none.base}/api/vault/sweep`, { method: "POST", headers: CSRF })).status).toBe(409)
		server?.stop()

		const sweepNow = vi.fn().mockResolvedValue(undefined)
		const redeemAll = vi.fn().mockResolvedValue(undefined)
		const reconfigure = vi.fn().mockResolvedValue(undefined)
		const { base } = await startServer({ vault: { sweepNow, redeemAll, reconfigure } })
		expect((await fetch(`${base}/api/vault/sweep`, { method: "POST", headers: CSRF })).status).toBe(200)
		expect(sweepNow).toHaveBeenCalledTimes(1)
		expect((await fetch(`${base}/api/vault/redeem`, { method: "POST", headers: CSRF })).status).toBe(200)
		expect(redeemAll).toHaveBeenCalledTimes(1)
	})

	it("reports rebalancing configuration and triggers", async () => {
		const unconfigured = await startServer()
		expect(await (await fetch(`${unconfigured.base}/api/rebalancing`)).json()).toEqual({ configured: false })
		server?.stop()

		const config = fakeConfig()
		config.rebalancing = { triggerPercentage: 0.5, baseBalances: { USDC: { "8453": "10000" } } }
		const checkTriggers = vi.fn().mockResolvedValue({ triggeredChains: [] })
		const { base } = await startServer({ config, rebalancing: { checkTriggers } })
		const res = await (await fetch(`${base}/api/rebalancing`)).json()
		expect(res.configured).toBe(true)
		expect(res.triggerPercentage).toBe(0.5)
		expect(res.triggers).toEqual({ triggeredChains: [] })
	})

	it("enables a disabled side on a curve-priced strategy and persists the curve", async () => {
		const { base, operator } = await startServer()
		// strategy 3 is ask-only; wire an enableSide like boot does for curve-priced FX
		const strategy3 = operator.strategies.find((s) => s.index === 3)!
		const enableSide = vi.fn()
		strategy3.enableSide = enableSide

		const newBid = [{ amount: "0", price: "1600" }]
		const res = await put(base, "/api/strategies/3/curves", { bidPriceCurve: newBid })
		expect(res.status).toBe(200)
		const bodyJson = await res.json()
		expect(bodyJson.bid).toEqual(newBid)
		expect(enableSide).toHaveBeenCalledTimes(1)
		expect(enableSide.mock.calls[0][0]).toBe("bid")
		expect(strategy3.bid?.getPoints()).toEqual(newBid)

		// invalid curve neither enables nor mutates
		const bad = await put(base, "/api/strategies/3/curves", { bidPriceCurve: [{ amount: "0", price: "-1" }] })
		expect(bad.status).toBe(400)
		expect(enableSide).toHaveBeenCalledTimes(1)
	})

	it("disables a side with an empty curve and persists the removal (one-sided LP)", async () => {
		const { base, operator } = await startServer()
		const strategy1 = operator.strategies.find((s) => s.index === 1)!
		const disableSide = vi.fn()
		strategy1.disableSide = disableSide

		const res = await put(base, "/api/strategies/1/curves", { askPriceCurve: [] })
		expect(res.status).toBe(200)
		const bodyJson = await res.json()
		expect(bodyJson.ask).toBeUndefined()
		expect(bodyJson.bid).toEqual(BID_POINTS)
		expect(disableSide).toHaveBeenCalledWith("ask")
		expect(strategy1.ask).toBeUndefined()
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.pairs?.[1]?.askPriceCurve).toBeUndefined()
		expect(written.pairs?.[1]?.bidPriceCurve).toBeDefined()
	})

	it("rejects disabling the only remaining side", async () => {
		const { base, operator } = await startServer()
		const strategy3 = operator.strategies.find((s) => s.index === 3)! // ask-only
		strategy3.disableSide = vi.fn()
		const res = await put(base, "/api/strategies/3/curves", { askPriceCurve: [] })
		expect(res.status).toBe(409)
		expect((await res.json()).error).toContain("at least one side")
		expect(strategy3.disableSide).not.toHaveBeenCalled()
	})

	it("rejects deleting the ask of a same-token market", async () => {
		const { base } = await startServer()
		const res = await put(base, "/api/strategies/0/curves", { askPriceCurve: [] })
		expect(res.status).toBe(409)
		expect((await res.json()).error).toContain("ask-only")
	})

	it("treats an empty curve on an already-absent side as a no-op", async () => {
		const { base } = await startServer()
		// strategy 3 is ask-only: empty bid does nothing, the ask update applies.
		const res = await put(base, "/api/strategies/3/curves", {
			bidPriceCurve: [],
			askPriceCurve: [{ amount: "0", price: "1500" }],
		})
		expect(res.status).toBe(200)
		expect((await res.json()).bid).toBeUndefined()
	})

	it("updates the vault set at runtime and persists it", async () => {
		const sweepNow = vi.fn().mockResolvedValue(undefined)
		const redeemAll = vi.fn().mockResolvedValue(undefined)
		const reconfigure = vi.fn().mockResolvedValue(undefined)
		const { base, operator } = await startServer({ vault: { sweepNow, redeemAll, reconfigure } })

		const vaults = [
			{ chain: "EVM-8453", vault: "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6", threshold: "5000", minBalance: "3000" },
		]
		const res = await fetch(`${base}/api/vault`, { method: "PUT", headers: CSRF, body: JSON.stringify({ vaults }) })
		expect(await res.json()).toEqual({ applied: true, restartNeeded: false, persisted: true })
		expect(reconfigure).toHaveBeenCalledWith(vaults, undefined)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.vault?.vaults).toEqual(vaults)

		// invalid rows rejected before any application
		const bad = await fetch(`${base}/api/vault`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ vaults: [{ chain: "EVM-8453", vault: "0xabc", threshold: "10", minBalance: "20" }] }),
		})
		expect(bad.status).toBe(400)
	})

	it("reports restart-needed when no vault venue exists at boot", async () => {
		const vaultPreflight = vi.fn().mockResolvedValue(undefined)
		const { base, operator } = await startServer({ vaultPreflight })
		const vaults = [{ chain: "EVM-8453", vault: "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6" }]
		const res = await fetch(`${base}/api/vault`, { method: "PUT", headers: CSRF, body: JSON.stringify({ vaults }) })
		expect(await res.json()).toEqual({ applied: false, restartNeeded: true, persisted: true })
		expect(vaultPreflight).toHaveBeenCalledWith(vaults)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.vault?.vaults).toEqual(vaults)
	})

	it("rejects a vault set that fails preflight instead of persisting it", async () => {
		const vaultPreflight = vi
			.fn()
			.mockRejectedValue(new Error("Vaults A and B on EVM-8453 share the underlying asset USDC"))
		const { base, operator } = await startServer({ vaultPreflight })
		const res = await fetch(`${base}/api/vault`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({
				vaults: [
					{ chain: "EVM-8453", vault: "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6" },
					{ chain: "EVM-8453", vault: "0x616a4E1db48e22028f6bbf20444Cd3b8e3273738" },
				],
			}),
		})
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("share the underlying asset")
		expect(existsSync(operator.configPath!)).toBe(false)
	})

	it("executes an operator send and surfaces the tx hash", async () => {
		const send = vi.fn().mockResolvedValue({ txHash: "0xabc123", sponsored: true, redeemed: false })
		const { base } = await startServer({ send })

		const body = {
			chain: "EVM-8453",
			token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
			amount: "25",
			to: "0x5b2c3e25243634732eE94525Ae98aEc404c82506",
		}
		const res = await fetch(`${base}/api/send`, { method: "POST", headers: CSRF, body: JSON.stringify(body) })
		expect(res.status).toBe(200)
		expect(await res.json()).toEqual({ txHash: "0xabc123", sponsored: true, redeemed: false })
		expect(send).toHaveBeenCalledWith(body)

		// invalid inputs rejected before reaching the sender
		for (const bad of [
			{ ...body, to: "0x123" },
			{ ...body, amount: "0" },
			{ ...body, amount: "" },
			{ chain: "EVM-8453" },
		]) {
			const rejected = await fetch(`${base}/api/send`, { method: "POST", headers: CSRF, body: JSON.stringify(bad) })
			expect(rejected.status).toBe(400)
		}
		expect(send).toHaveBeenCalledTimes(1)

		// sender errors surface as 400 with the message
		send.mockRejectedValueOnce(new Error("Insufficient token balance"))
		const failed = await fetch(`${base}/api/send`, { method: "POST", headers: CSRF, body: JSON.stringify(body) })
		expect(failed.status).toBe(400)
		expect((await failed.json()).error).toContain("Insufficient")
	})

	it("does not list vault shares among send token options", async () => {
		const vaultAddress = "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"
		const config = fakeConfig()
		config.vault = { vaults: [{ chain: "EVM-8453", vault: vaultAddress }] }
		const { base } = await startServer({ config })
		const configDto = await (await fetch(`${base}/api/config`)).json()
		const tokens = configDto.sendTokens["EVM-8453"] as Array<{ address: string; symbol: string }>
		expect(tokens[0]).toEqual({ symbol: "native", address: "native" })
		expect(tokens.some((t) => t.address.toLowerCase() === vaultAddress.toLowerCase())).toBe(false)
		expect(configDto.knownVaults["EVM-8453"].length).toBeGreaterThan(0)
	})

	it("records operator sends in the wallet history and merges fill txs", async () => {
		const send = vi.fn().mockResolvedValue({ txHash: "0xabc123", sponsored: true, redeemed: true })
		const { base, operator } = await startServer({ send })
		const activity = operator.data.activity
		await activity.record({ type: "filled", orderId: "0xorder", txHash: "0xf1", chainId: 56 })
		await activity.record({ type: "skipped", orderId: "0xother", reason: "unprofitable" }) // no tx hash — not history

		const body = {
			chain: "EVM-8453",
			token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
			amount: "25",
			to: "0x5b2c3e25243634732eE94525Ae98aEc404c82506",
		}
		const res = await fetch(`${base}/api/send`, { method: "POST", headers: CSRF, body: JSON.stringify(body) })
		expect(res.status).toBe(200)

		const history = await (await fetch(`${base}/api/wallet/history`)).json()
		expect(history.txs).toHaveLength(2)
		expect(history.txs.find((t: { kind: string }) => t.kind === "send")).toMatchObject({
			chainId: 8453,
			token: "USDC",
			amount: "25",
			to: body.to,
			txHash: "0xabc123",
			sponsored: true,
		})
		expect(history.txs.find((t: { kind: string }) => t.kind === "fill")).toMatchObject({
			chainId: 56,
			txHash: "0xf1",
		})
	})

	it("updates rebalancing settings at runtime and persists them", async () => {
		const checkTriggers = vi.fn().mockResolvedValue({ triggeredChains: [] })
		const { base, operator } = await startServer({ rebalancing: { checkTriggers } })

		const body = { triggerPercentage: 0.4, baseBalances: { USDC: { "8453": "12000" } } }
		const res = await fetch(`${base}/api/rebalancing`, { method: "PUT", headers: CSRF, body: JSON.stringify(body) })
		expect(await res.json()).toEqual({ applied: true, restartNeeded: false, persisted: true })
		expect(operator.applyRebalancing).toHaveBeenCalledWith({
			triggerPercentage: 0.4,
			baseBalances: { USDC: { "8453": "12000" } },
		})
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.rebalancing?.triggerPercentage).toBe(0.4)

		const badTrigger = await fetch(`${base}/api/rebalancing`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ triggerPercentage: 1.5, baseBalances: { USDC: { "1": "10" } } }),
		})
		expect(badTrigger.status).toBe(400)
	})

	it("serves the masked running config", async () => {
		const { base } = await startServer()
		const res = await (await fetch(`${base}/api/config`)).json()
		expect(res.toml).toContain("[simplex.signer]")
		expect(res.toml).not.toContain('key = "0xab"')
		expect(res.logLevel).toBe("info")
	})

	// A pairs set that passes whole-array validation — fakeConfig's default set
	// deliberately mirrors the strategy fixtures and is not itself valid.
	function marketConfig(): FillerTomlConfig {
		const config = fakeConfig()
		config.pairs = [
			{ token0: "USDC", token1: "USDC", maxOrderSize: "100000", askPriceCurve: SAME_ASSET_POINTS },
			{ token0: "USDC", token1: "CNGN", maxOrderSize: "5000", bidPriceCurve: BID_POINTS, askPriceCurve: ASK_POINTS },
		]
		return config
	}

	it("adds a market at runtime: hydrated via the capability and persisted", async () => {
		const addPair = vi.fn().mockReturnValue({
			index: 7,
			pairIndex: 2,
			exotic: "USDC/EURC",
			token0: "USDC",
			token1: "EURC",
			sameToken: false,
			referenceOnly: false,
		})
		const { base, operator } = await startServer({ config: marketConfig(), addPair })
		const body = {
			token0: "USDC",
			token1: "EURC",
			maxOrderSize: "5000",
			askPriceCurve: [{ amount: "0", price: "0.92" }],
		}
		const res = await fetch(`${base}/api/strategies`, { method: "POST", headers: CSRF, body: JSON.stringify(body) })
		expect(res.status).toBe(200)
		const payload = await res.json()
		expect(payload.applied).toBe(true)
		expect(payload.restartNeeded).toBe(false)
		expect(payload.strategy.index).toBe(7)
		expect(addPair).toHaveBeenCalledWith(body, undefined, 2)
		expect(operator.config.pairs).toHaveLength(3)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.pairs?.[2]?.token1).toBe("EURC")
	})

	it("rejects identical and reverse-orientation markets with nothing persisted", async () => {
		const addPair = vi.fn()
		const { base, operator } = await startServer({ config: marketConfig(), addPair })
		for (const [token0, token1] of [
			["USDC", "CNGN"],
			["CNGN", "USDC"],
		]) {
			const res = await fetch(`${base}/api/strategies`, {
				method: "POST",
				headers: CSRF,
				body: JSON.stringify({ token0, token1, maxOrderSize: "5000", askPriceCurve: ASK_POINTS }),
			})
			expect(res.status).toBe(400)
			expect((await res.json()).error).toMatch(/declared twice|already declared/)
		}
		expect(addPair).not.toHaveBeenCalled()
		expect(operator.config.pairs).toHaveLength(2)
		expect(existsSync(operator.configPath!)).toBe(false)
	})

	it("rejects an unanchored market with the validator's message", async () => {
		const { base } = await startServer({ config: marketConfig(), addPair: vi.fn() })
		const res = await fetch(`${base}/api/strategies`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({
				token0: "EURC",
				token1: "ZARP",
				maxOrderSize: "5000",
				askPriceCurve: [{ amount: "0", price: "19.4" }],
			}),
		})
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("no USD anchor")
	})

	it("adds a custom-token market, persisting its [assets] entry", async () => {
		const addPair = vi.fn().mockReturnValue(null)
		const { base, operator } = await startServer({ config: marketConfig(), addPair })
		const assets = { BRZ: { "EVM-8453": "0x5555555555555555555555555555555555555555" } }
		const res = await fetch(`${base}/api/strategies`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({
				token0: "USDC",
				token1: "BRZ",
				maxOrderSize: "5000",
				askPriceCurve: [{ amount: "0", price: "5.6" }],
				assets,
			}),
		})
		expect(res.status).toBe(200)
		expect(addPair).toHaveBeenCalledTimes(1)
		expect(addPair.mock.calls[0][1]).toEqual(assets)
		expect(operator.config.assets?.BRZ).toEqual(assets.BRZ)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.assets?.BRZ?.["EVM-8453"]).toBe("0x5555555555555555555555555555555555555555")
	})

	it("persists a market for restart when live hydration is unavailable", async () => {
		const { base, operator } = await startServer({ config: marketConfig() })
		const res = await fetch(`${base}/api/strategies`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({
				token0: "USDC",
				token1: "EURC",
				maxOrderSize: "5000",
				askPriceCurve: [{ amount: "0", price: "0.92" }],
			}),
		})
		expect(res.status).toBe(200)
		const payload = await res.json()
		expect(payload.applied).toBe(false)
		expect(payload.restartNeeded).toBe(true)
		expect(operator.config.pairs).toHaveLength(3)
	})

	it("removes a market at runtime and remaps later strategies to the right config rows", async () => {
		const config = marketConfig()
		config.pairs!.push({
			token0: "USDC",
			token1: "ZARP",
			maxOrderSize: "5000",
			askPriceCurve: [{ amount: "0", price: "17.8" }],
		})
		const zarpAsk = new FillerPricePolicy({ points: [{ amount: "0", price: "17.8" }] })
		const strategies: AdminStrategy[] = [
			{
				index: 0,
				pairIndex: 0,
				exotic: "USDC/USDC",
				token0: "USDC",
				token1: "USDC",
				ask: new FillerPricePolicy({ points: SAME_ASSET_POINTS }),
				sameToken: true,
			},
			{
				index: 1,
				pairIndex: 1,
				exotic: "USDC/CNGN",
				token0: "USDC",
				token1: "CNGN",
				bid: new FillerPricePolicy({ points: BID_POINTS }),
				ask: new FillerPricePolicy({ points: ASK_POINTS }),
				sameToken: false,
			},
			{ index: 2, pairIndex: 2, exotic: "USDC/ZARP", token0: "USDC", token1: "ZARP", ask: zarpAsk, sameToken: false },
		]
		const removePair = vi.fn(async (index: number) => {
			const position = strategies.findIndex((s) => s.index === index)
			const { pairIndex } = strategies[position]
			strategies.splice(position, 1)
			for (const s of strategies) {
				if (s.pairIndex > pairIndex) s.pairIndex -= 1
			}
		})
		const { base, operator } = await startServer({ config, strategies, removePair })
		const res = await fetch(`${base}/api/strategies/1`, { method: "DELETE", headers: CSRF })
		expect(res.status).toBe(200)
		expect((await res.json()).applied).toBe(true)
		expect(removePair).toHaveBeenCalledWith(1)
		expect(operator.config.pairs).toHaveLength(2)

		// The ZARP strategy (index 2) now maps to config row 1 — a curve edit must land on the right pair.
		const put = await fetch(`${base}/api/strategies/2/curves`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ askPriceCurve: [{ amount: "0", price: "18" }] }),
		})
		expect(put.status).toBe(200)
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.pairs).toHaveLength(2)
		expect(written.pairs?.[1]?.token1).toBe("ZARP")
		expect(written.pairs?.[1]?.askPriceCurve?.[0]?.price).toBe("18")
	})

	it("refuses removals that orphan an anchor, empty the market list, or name an unknown strategy", async () => {
		const config = marketConfig()
		config.pairs = [
			{ token0: "USDC", token1: "ZARP", referenceOnly: true, askPriceCurve: [{ amount: "0", price: "17.8" }] },
			{ token0: "ZARP", token1: "CNGN", maxOrderSize: "90000", askPriceCurve: [{ amount: "0", price: "5.2" }] },
		]
		const strategies: AdminStrategy[] = [
			{
				index: 0,
				pairIndex: 0,
				exotic: "USDC/ZARP (reference)",
				token0: "USDC",
				token1: "ZARP",
				ask: new FillerPricePolicy({ points: [{ amount: "0", price: "17.8" }] }),
				sameToken: false,
				referenceOnly: true,
			},
			{
				index: 1,
				pairIndex: 1,
				exotic: "ZARP/CNGN",
				token0: "ZARP",
				token1: "CNGN",
				ask: new FillerPricePolicy({ points: [{ amount: "0", price: "5.2" }] }),
				sameToken: false,
			},
		]
		const removePair = vi.fn()
		const { base, operator } = await startServer({ config, strategies, removePair })

		// Removing the reference feed would orphan ZARP/CNGN's USD anchor.
		const orphan = await fetch(`${base}/api/strategies/0`, { method: "DELETE", headers: CSRF })
		expect(orphan.status).toBe(400)
		expect((await orphan.json()).error).toContain("no USD anchor")

		const missing = await fetch(`${base}/api/strategies/99`, { method: "DELETE", headers: CSRF })
		expect(missing.status).toBe(404)
		expect(removePair).not.toHaveBeenCalled()
		expect(operator.config.pairs).toHaveLength(2)

		// A single-market config: the last market cannot be removed live.
		server?.stop()
		const single = marketConfig()
		single.pairs = [single.pairs![1]]
		const { base: base2 } = await startServer({
			config: single,
			strategies: [
				{
					index: 0,
					pairIndex: 0,
					exotic: "USDC/CNGN",
					token0: "USDC",
					token1: "CNGN",
					bid: new FillerPricePolicy({ points: BID_POINTS }),
					ask: new FillerPricePolicy({ points: ASK_POINTS }),
					sameToken: false,
				},
			],
			removePair,
		})
		const last = await fetch(`${base2}/api/strategies/0`, { method: "DELETE", headers: CSRF })
		expect(last.status).toBe(400)
		expect((await last.json()).error).toContain("cannot be removed")
		expect(removePair).not.toHaveBeenCalled()
	})

	it("serves static SPA files with an index.html fallback", async () => {
		const uiDistDir = mkdtempSync(join(tmpdir(), "simplex-dist-"))
		writeFileSync(join(uiDistDir, "index.html"), "<html>spa</html>")
		mkdirSync(join(uiDistDir, "assets"))
		writeFileSync(join(uiDistDir, "assets", "app.js"), "console.log(1)")

		server = new UiServer({ mode: "operator", uiDistDir, operator: baseOperator() })
		const port = await server.start(0)
		const base = `http://127.0.0.1:${port}`

		expect(await (await fetch(base)).text()).toBe("<html>spa</html>")
		const js = await fetch(`${base}/assets/app.js`)
		expect(js.headers.get("content-type")).toContain("text/javascript")
		// client-routed path falls back to the SPA shell
		expect(await (await fetch(`${base}/setup/step-2`)).text()).toBe("<html>spa</html>")
		// traversal is blocked (fetch normalizes ../, so send the raw path over a socket)
		const traversal = await rawRequest(port, "/../secret.txt")
		expect(traversal).toContain("403")
	})

	it("resizes a market's per-order cap on the live pair and persists it", async () => {
		const { base, operator } = await startServer({ config: marketConfig() })
		const setMaxOrderSize = vi.fn()
		operator.strategies.find((s) => s.index === 1)!.setMaxOrderSize = setMaxOrderSize

		const res = await put(base, "/api/strategies/1", { maxOrderSize: "12500" })
		expect(res.status).toBe(200)
		const payload = await res.json()
		expect(payload.applied).toBe(true)
		expect(payload.restartNeeded).toBe(false)
		expect(payload.maxOrderSize).toBe("12500")
		expect(setMaxOrderSize).toHaveBeenCalledWith("12500")
		expect(operator.config.pairs?.[1]?.maxOrderSize).toBe("12500")
		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.pairs?.[1]?.maxOrderSize).toBe("12500")
	})

	it("persists a cap for restart when the pair cannot be resized live", async () => {
		const { base, operator } = await startServer({ config: marketConfig() })
		// No setMaxOrderSize hook: no engine ran at boot.
		const res = await put(base, "/api/strategies/1", { maxOrderSize: "7500" })
		expect(await res.json()).toMatchObject({ applied: false, restartNeeded: true, persisted: true })
		expect(operator.config.pairs?.[1]?.maxOrderSize).toBe("7500")
	})

	it("rejects non-positive caps and reference-only markets, leaving the pair untouched", async () => {
		const { base, operator } = await startServer({ config: marketConfig() })
		const strategy = operator.strategies.find((s) => s.index === 1)!
		strategy.setMaxOrderSize = vi.fn()

		for (const value of ["0", "-5", "abc", ""]) {
			const res = await put(base, "/api/strategies/1", { maxOrderSize: value })
			expect(res.status).toBe(400)
			expect((await res.json()).error).toMatch(/maxOrderSize must be a (positive number|decimal string)/)
		}
		const unknown = await put(base, "/api/strategies/1", { maxOrderSize: "1", token0: "USDC" })
		expect(unknown.status).toBe(400)
		expect((await unknown.json()).error).toContain("Unknown fields")

		strategy.referenceOnly = true
		const reference = await put(base, "/api/strategies/1", { maxOrderSize: "1000" })
		expect(reference.status).toBe(409)
		expect((await reference.json()).error).toContain("never fill")

		expect(strategy.setMaxOrderSize).not.toHaveBeenCalled()
		expect(operator.config.pairs?.[1]?.maxOrderSize).toBe("5000")
		expect(existsSync(operator.configPath!)).toBe(false)
	})

	/** Two chain rows aligned with the operator's running chain ids. */
	function chainsConfig(): FillerTomlConfig {
		const config = marketConfig()
		config.chains = [
			{ rpcUrls: ["https://base.example"], bundlerUrl: "https://base-bundler.example" },
			{ rpcUrls: ["https://bsc.example"], bundlerUrl: "https://bsc-bundler.example" },
		]
		return config
	}

	it("lists the configured chains alongside the wizard's catalog for their network", async () => {
		const { base } = await startServer({ config: chainsConfig() })
		const dto = await (await fetch(`${base}/api/chains`)).json()
		expect(dto.chains).toEqual([
			{
				chainId: 8453,
				stateMachineId: "EVM-8453",
				label: "Base",
				rpcUrls: ["https://base.example"],
				bundlerUrl: "https://base-bundler.example",
				watchOnly: false,
				running: true,
			},
			{
				chainId: 56,
				stateMachineId: "EVM-56",
				label: "BNB Chain",
				rpcUrls: ["https://bsc.example"],
				bundlerUrl: "https://bsc-bundler.example",
				watchOnly: false,
				running: true,
			},
		])
		expect(dto.globalWatchOnly).toBe(false)
		// The catalog is every selectable chain on the network — configured ones
		// included, since the editor toggles them in place like the wizard does.
		expect(dto.network).toBe("mainnet")
		const catalogIds = dto.catalog.map((c: { chainId: number }) => c.chainId)
		expect(catalogIds).toEqual(expect.arrayContaining([1, 8453, 56, 42161, 137]))
		expect(catalogIds).not.toContain(84532)
	})

	it("scopes the catalog to testnet when the configured chains are testnets", async () => {
		const config = chainsConfig()
		const { base } = await startServer({ config, chains: [84532, 11155111] })
		const dto = await (await fetch(`${base}/api/chains`)).json()
		expect(dto.network).toBe("testnet")
		expect(dto.catalog.map((c: { chainId: number }) => c.chainId)).toEqual(
			expect.arrayContaining([84532, 11155111]),
		)
		expect(dto.catalog.map((c: { chainId: number }) => c.chainId)).not.toContain(1)
	})

	it("replaces the chain set: probes only new endpoints, persists, and asks for a restart", async () => {
		const fetchChainId = vi.fn(async (url: string) => (url.includes("arb") ? 42161 : 8453))
		const { base, operator } = await startServer({ config: chainsConfig() }, { fetchChainId })
		// Drop BNB Chain, add Arbitrum, and give Base a second quorum provider.
		const res = await put(base, "/api/chains", {
			chains: [
				{
					chainId: 8453,
					rpcUrls: ["https://base.example", "https://base-two.example"],
					bundlerUrl: "https://base-bundler.example",
				},
				{ chainId: 42161, rpcUrls: ["https://arb.example"], bundlerUrl: "https://arb-bundler.example", watchOnly: true },
			],
		})
		expect(res.status).toBe(200)
		expect(await res.json()).toMatchObject({ applied: false, restartNeeded: true, persisted: true, removed: [56] })
		// The pre-existing Base endpoint is not re-probed; the two new ones are.
		expect(fetchChainId.mock.calls.map((c) => c[0]).sort()).toEqual([
			"https://arb.example",
			"https://base-two.example",
		])
		expect(operator.config.chains).toHaveLength(2)
		expect(operator.config.simplex.watchOnly).toEqual({ "42161": true })

		const written = parse(readFileSync(operator.configPath!, "utf-8")) as FillerTomlConfig
		expect(written.chains[1].bundlerUrl).toBe("https://arb-bundler.example")
		// The rewritten file keeps the chain rows identifiable — the TOML has no chain id.
		expect(readFileSync(operator.configPath!, "utf-8")).toContain("# Arbitrum")

		// The new row is reported as configured but not yet live.
		const dto = await (await fetch(`${base}/api/chains`)).json()
		expect(dto.chains.map((c: { chainId: number; running: boolean }) => [c.chainId, c.running])).toEqual([
			[8453, true],
			[42161, false],
		])
	})

	it("rejects chain edits that boot would reject, with nothing persisted", async () => {
		const fetchChainId = vi.fn(async () => 999)
		const { base, operator } = await startServer({ config: chainsConfig() }, { fetchChainId })
		const base8453 = { chainId: 8453, rpcUrls: ["https://base.example"], bundlerUrl: "https://base-bundler.example" }

		const empty = await put(base, "/api/chains", { chains: [] })
		expect((await empty.json()).error).toContain("at least one chain")

		const sameHost = await put(base, "/api/chains", {
			chains: [{ ...base8453, rpcUrls: ["https://base.example", "https://base.example/two"] }],
		})
		expect((await sameHost.json()).error).toContain("different domains")

		const noBundler = await put(base, "/api/chains", { chains: [{ ...base8453, bundlerUrl: "  " }] })
		expect((await noBundler.json()).error).toContain("bundler URL")

		const uncovered = await put(base, "/api/chains", {
			chains: [base8453, { chainId: 999, rpcUrls: ["https://odd.example"], bundlerUrl: "https://odd.example" }],
		})
		expect((await uncovered.json()).error).toContain("No confirmation policy")

		const wrongChain = await put(base, "/api/chains", {
			chains: [{ ...base8453, rpcUrls: ["https://impostor.example"] }],
		})
		expect((await wrongChain.json()).error).toContain("reports chain 999, expected 8453")

		expect(operator.config.chains).toHaveLength(2)
		expect(existsSync(operator.configPath!)).toBe(false)
	})

	it("refuses to drop a chain that still holds a funding venue", async () => {
		const config = chainsConfig()
		config.vault = { vaults: [{ chain: "EVM-56", vault: "0x1111111111111111111111111111111111111111" }] }
		const { base, operator } = await startServer({ config }, { fetchChainId: vi.fn(async () => 8453) })
		const res = await put(base, "/api/chains", {
			chains: [{ chainId: 8453, rpcUrls: ["https://base.example"], bundlerUrl: "https://base-bundler.example" }],
		})
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("vault treasury")
		expect(operator.config.chains).toHaveLength(2)
	})

	it("writes a testnet confirmation policy for an added testnet chain", async () => {
		const { base, operator } = await startServer({ config: chainsConfig() }, { fetchChainId: vi.fn(async () => 84532) })
		const res = await put(base, "/api/chains", {
			chains: [
				{ chainId: 8453, rpcUrls: ["https://base.example"], bundlerUrl: "https://base-bundler.example" },
				{ chainId: 84532, rpcUrls: ["https://base-sepolia.example"], bundlerUrl: "https://bundler.example" },
			],
		})
		expect(res.status).toBe(200)
		expect(operator.config.confirmationPolicies?.["84532"]?.points.length).toBeGreaterThan(0)
	})

	it("keeps a global watch-only switch intact when the chain set changes", async () => {
		const config = chainsConfig()
		config.simplex.watchOnly = true
		const { base, operator } = await startServer({ config }, { fetchChainId: vi.fn(async () => 8453) })
		const dto = await (await fetch(`${base}/api/chains`)).json()
		expect(dto.globalWatchOnly).toBe(true)
		expect(dto.chains.every((c: { watchOnly: boolean }) => c.watchOnly)).toBe(true)

		await put(base, "/api/chains", {
			chains: [{ chainId: 8453, rpcUrls: ["https://base.example"], bundlerUrl: "https://base-bundler.example" }],
		})
		// Expanding it per chain would stop validateConfig treating this as an
		// all-watch-only (signer-less) config.
		expect(operator.config.simplex.watchOnly).toBe(true)
	})

	it("serves the wizard's stateless probes in operator mode but not its stateful routes", async () => {
		// The Alchemy check probes the first derived URL, which is Ethereum's.
		const fetchChainId = vi.fn(async (url: string) => (url.includes("eth-mainnet") ? 1 : 8453))
		const { base } = await startServer({ config: chainsConfig() }, { fetchChainId })
		const rpc = await fetch(`${base}/api/setup/validate-rpc`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ urls: ["https://base.example"], expectedChainId: 8453 }),
		})
		expect(await rpc.json()).toMatchObject({ ok: true })

		// The chain editor prefills endpoints from one Alchemy key, like the wizard.
		const alchemy = await fetch(`${base}/api/setup/validate-alchemy-key`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ apiKey: "test-key", network: "mainnet" }),
		})
		const prefill = await alchemy.json()
		expect(prefill.valid).toBe(true)
		const base8453 = prefill.chains.find((c: { chainId: number }) => c.chainId === 8453)
		expect(base8453.rpcUrl).toContain("test-key")
		// Alchemy serves ERC-4337 bundler methods on the same endpoint.
		expect(base8453.bundlerUrl).toBe(base8453.rpcUrl)

		expect((await fetch(`${base}/api/setup/defaults`)).status).toBe(410)
		expect((await fetch(`${base}/api/setup/save-and-start`, { method: "POST", headers: CSRF, body: "{}" })).status).toBe(410)
	})

})

describe("UiServer (init mode)", () => {
	let server: UiServer | undefined

	afterEach(() => {
		server?.stop()
		server = undefined
	})

	it("refuses non-loopback binds", async () => {
		server = new UiServer({
			mode: "init",
			setup: { configPath: "/tmp/x.toml", onSaveAndStart: async () => {} },
		})
		await expect(server.start(0, "0.0.0.0")).rejects.toThrow(/loopback/)
	})

	it("reports init status and gates operator endpoints", async () => {
		server = new UiServer({
			mode: "init",
			setup: { configPath: "/tmp/x.toml", onSaveAndStart: async () => {} },
		})
		const port = await server.start(0)
		const base = `http://127.0.0.1:${port}`

		expect(await (await fetch(`${base}/api/status`)).json()).toEqual({ mode: "init", starting: false })
		expect((await fetch(`${base}/api/strategies`)).status).toBe(409)
		expect((await fetch(`${base}/api/balances`)).status).toBe(409)
		expect((await fetch(`${base}/api/pause`, { method: "POST", headers: CSRF })).status).toBe(409)
	})

	it("enterOperatorMode flips the live server", async () => {
		server = new UiServer({
			mode: "init",
			setup: { configPath: "/tmp/x.toml", onSaveAndStart: async () => {} },
		})
		const port = await server.start(0)
		const base = `http://127.0.0.1:${port}`

		server.enterOperatorMode(baseOperator({ chains: [1], strategyTypes: ["stable"] }))

		const status = await (await fetch(`${base}/api/status`)).json()
		expect(status.mode).toBe("operator")
		const startStatus = await (await fetch(`${base}/api/setup/start-status`)).json()
		expect(startStatus.state).toBe("running")
		expect((await fetch(`${base}/api/setup/defaults`)).status).toBe(410)
	})
})
