import { AdminServer } from "@/services/server/AdminServer"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { describe, it, expect, afterEach } from "vitest"
import Decimal from "decimal.js"

// Covers inflight price curve changes: in-place mutation on FillerPricePolicy and
// the AdminServer HTTP surface that exposes it. The server holds the same policy
// instances the strategies price with, so replacing points changes live pricing.

const BID_POINTS = [
	{ amount: "100", price: "1580" },
	{ amount: "5000", price: "1570" },
]
const ASK_POINTS = [
	{ amount: "100", price: "1560" },
	{ amount: "5000", price: "1550" },
]

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

describe("AdminServer", () => {
	let server: AdminServer | undefined

	afterEach(() => {
		server?.stop()
		server = undefined
	})

	async function startServer() {
		const bid = new FillerPricePolicy({ points: BID_POINTS })
		const ask = new FillerPricePolicy({ points: ASK_POINTS })
		const askOnly = new FillerPricePolicy({ points: ASK_POINTS })
		server = new AdminServer([
			{ index: 1, exotic: "cNGN", bid, ask },
			{ index: 2 }, // venue-priced: no editable curves
			{ index: 3, ask: askOnly }, // one-sided LP
		])
		const port = await server.start(0)
		return { base: `http://127.0.0.1:${port}`, bid, ask, askOnly }
	}

	async function put(base: string, path: string, body: unknown) {
		return fetch(`${base}${path}`, {
			method: "PUT",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(body),
		})
	}

	it("serves the UI and health check", async () => {
		const { base } = await startServer()
		const ui = await fetch(base)
		expect(ui.status).toBe(200)
		expect(ui.headers.get("content-type")).toContain("text/html")
		expect(await ui.text()).toContain("Price Curves")

		const health = await fetch(`${base}/health`)
		expect(await health.json()).toEqual({ status: "ok" })
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

	it("applies a curve update to the live policy instance and returns the new state", async () => {
		const { base, bid, ask } = await startServer()
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
		})
		expect(ask.getPoints()).toEqual(newAsk)
		expect(bid.getPoints()).toEqual(BID_POINTS)
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
		expect((await fetch(`${base}/api/strategies`, { method: "POST" })).status).toBe(405)
	})
})
