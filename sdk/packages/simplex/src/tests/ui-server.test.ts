import { UiServer, type HaltControl, type OperatorContext, type PauseControl } from "@/services/server/UiServer"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import type { FillerTomlConfig } from "@/config/filler-toml"
import { loadRuntimeState } from "@/core/runtime-state"
import { SignerType } from "@/services/wallet"
import { describe, it, expect, afterEach, vi } from "vitest"
import { existsSync, mkdtempSync, readFileSync, writeFileSync, mkdirSync } from "fs"
import { createConnection } from "net"
import { tmpdir } from "os"
import { join } from "path"
import { parse } from "toml"
import Decimal from "decimal.js"

/** fetch() normalizes `..` out of URLs, so traversal tests need a raw socket. */
function rawRequest(port: number, path: string): Promise<string> {
	return new Promise((resolve, reject) => {
		const socket = createConnection(port, "127.0.0.1", () => {
			socket.write(`GET ${path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n`)
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

/** strategies[] indices line up with the AdminStrategy indices used in the tests. */
function fakeConfig(): FillerTomlConfig {
	return {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0xab" },
			maxConcurrentOrders: 5,
			queue: { maxRechecks: 10, recheckDelayMs: 30000 },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example",
		},
		strategies: [
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 100 },
					{ amount: "100000", value: 10 },
				],
			},
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: { "EVM-56": "0x1111111111111111111111111111111111111111" },
				bidPriceCurve: BID_POINTS,
				askPriceCurve: ASK_POINTS,
			},
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: { "EVM-56": "0x1111111111111111111111111111111111111111" },
			},
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: { "EVM-56": "0x1111111111111111111111111111111111111111" },
				askPriceCurve: ASK_POINTS,
			},
		],
		chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
	}
}

function baseOperator(overrides: Partial<OperatorContext> = {}): OperatorContext {
	const dataDir = mkdtempSync(join(tmpdir(), "simplex-ui-"))
	return {
		strategies: [],
		filler: fakePauseControl(),
		balances: { getSnapshot: () => ({ updatedAt: null, chains: [] }) },
		haltControls: [],
		config: fakeConfig(),
		stop: vi.fn().mockResolvedValue(undefined),
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(dataDir, "filler-config.toml"),
		chains: [8453, 56],
		strategyTypes: ["hyperfx"],
		dataDir,
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

	it("updatePrice flattens the curve to a single price", () => {
		const policy = new FillerPricePolicy({ points: BID_POINTS })
		policy.updatePrice(new Decimal(1620))
		expect(policy.getPoints()).toEqual([{ amount: "0", price: "1620" }])
	})
})

describe("UiServer (operator mode)", () => {
	let server: UiServer | undefined

	afterEach(() => {
		server?.stop()
		server = undefined
	})

	async function startServer(overrides: Partial<OperatorContext> = {}) {
		const bid = new FillerPricePolicy({ points: BID_POINTS })
		const ask = new FillerPricePolicy({ points: ASK_POINTS })
		const askOnly = new FillerPricePolicy({ points: ASK_POINTS })
		const filler = fakePauseControl()
		const operator = baseOperator({
			strategies: [
				{ index: 1, exotic: "cNGN", bid, ask },
				{ index: 2 }, // venue-priced: no editable curves
				{ index: 3, ask: askOnly }, // one-sided LP
			],
			filler,
			balances: { getSnapshot: () => ({ updatedAt: 123, chains: [{ chainId: 8453, usdc: 1500 }] }) },
			...overrides,
		})
		server = new UiServer({ mode: "operator", operator })
		const port = await server.start(0)
		return {
			base: `http://127.0.0.1:${port}`,
			bid,
			ask,
			askOnly,
			filler,
			dataDir: operator.dataDir!,
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
		expect(payload.strategyTypes).toEqual(["hyperfx"])
	})

	it("rejects mutating requests without the X-Simplex-UI header", async () => {
		const { base, bid } = await startServer()
		const res = await put(base, "/api/strategies/1/curves", { bidPriceCurve: BID_POINTS }, {})
		expect(res.status).toBe(403)
		expect(bid.getPoints()).toEqual(BID_POINTS)

		const pause = await fetch(`${base}/api/pause`, { method: "POST" })
		expect(pause.status).toBe(403)
	})

	it("lists strategies with their curves", async () => {
		const { base } = await startServer()
		const res = await fetch(`${base}/api/strategies`)
		expect(res.status).toBe(200)
		expect(await res.json()).toEqual({
			strategies: [
				{ index: 1, exotic: "cNGN", pricingMode: "static", bid: BID_POINTS, ask: ASK_POINTS },
				{ index: 2, pricingMode: "venue" },
				{ index: 3, pricingMode: "static", ask: ASK_POINTS },
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
			exotic: "cNGN",
			pricingMode: "static",
			bid: BID_POINTS,
			ask: newAsk,
			persisted: true,
		})
		expect(ask.getPoints()).toEqual(newAsk)
		expect(bid.getPoints()).toEqual(BID_POINTS)

		// restarts keep the change: the config file now carries the new curve
		expect(existsSync(operator.configPath)).toBe(true)
		const written = parse(readFileSync(operator.configPath, "utf-8")) as FillerTomlConfig
		const fx = written.strategies[1]
		if (fx.type !== "hyperfx") throw new Error("expected hyperfx at index 1")
		expect(fx.askPriceCurve).toEqual(newAsk)
		expect(fx.bidPriceCurve).toEqual(BID_POINTS)
	})

	it("rejects malformed bodies with 400", async () => {
		const { base } = await startServer()
		expect((await put(base, "/api/strategies/1/curves", {})).status).toBe(400)
		expect((await put(base, "/api/strategies/1/curves", { bidPriceCurve: "flat" })).status).toBe(400)
		expect((await put(base, "/api/strategies/1/curves", { bidPriceCurve: [] })).status).toBe(400)
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
		expect((await fetch(`${base}/api/strategies`, { method: "POST", headers: CSRF })).status).toBe(405)
	})

	it("pause/resume toggles the filler and persists the state", async () => {
		const { base, filler, dataDir } = await startServer()

		const pause = await fetch(`${base}/api/pause`, { method: "POST", headers: CSRF })
		expect(await pause.json()).toEqual({ paused: true })
		expect(filler.paused).toBe(true)
		expect(loadRuntimeState(dataDir)).toEqual({ paused: true })

		const resume = await fetch(`${base}/api/resume`, { method: "POST", headers: CSRF })
		expect(await resume.json()).toEqual({ paused: false })
		expect(filler.paused).toBe(false)
		expect(loadRuntimeState(dataDir)).toEqual({ paused: false })
	})

	it("serves the balance snapshot", async () => {
		const { base } = await startServer()
		const res = await fetch(`${base}/api/balances`)
		expect(await res.json()).toEqual({ updatedAt: 123, chains: [{ chainId: 8453, usdc: 1500 }] })
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

	it("serves a placeholder page when the UI is not built", async () => {
		const { base } = await startServer()
		const res = await fetch(base)
		expect(await res.text()).toContain("UI not built")
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
