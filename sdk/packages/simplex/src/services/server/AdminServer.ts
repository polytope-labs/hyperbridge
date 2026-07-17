import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { FillerPricePolicy, type PriceCurvePoint } from "@/config/interpolated-curve"
import { getLogger } from "../Logger"
import { ADMIN_UI_HTML } from "./ui/admin-ui"

/**
 * An FX strategy's editable price curves. The policies are the same instances
 * the running strategy prices with, so `replacePoints` takes effect on the next
 * order evaluation. A side is absent when it cannot be edited: disabled
 * (one-sided LP) or venue-priced (both sides absent).
 */
export interface AdminStrategy {
	/** Position in the TOML `strategies` array; stable identifier for the API. */
	index: number
	/** Exotic token symbol, display-only; distinguishes strategies when several FX markets run at once. */
	exotic?: string
	bid?: FillerPricePolicy
	ask?: FillerPricePolicy
}

const MAX_BODY_BYTES = 1_048_576

/**
 * Local HTTP server for inflight operational changes — currently price curve
 * updates on FX strategies, applied in memory without a restart. Serves a
 * single-page UI at `/` and a JSON API under `/api`. No authentication:
 * bind it to localhost or an otherwise trusted interface.
 */
export class AdminServer {
	private server: Server
	private logger = getLogger("admin")
	private strategies: AdminStrategy[]

	constructor(strategies: AdminStrategy[]) {
		this.strategies = strategies
		this.server = createServer((req, res) => {
			this.handle(req, res).catch((err) => {
				this.logger.error({ err }, "Unhandled admin request error")
				if (!res.headersSent) {
					this.json(res, 500, { error: "Internal server error" })
				}
			})
		})
	}

	/** Resolves with the bound port once listening (pass port 0 for an ephemeral port). */
	start(port: number, host = "127.0.0.1"): Promise<number> {
		return new Promise((resolve, reject) => {
			this.server.once("error", reject)
			this.server.listen(port, host, () => {
				const address = this.server.address()
				const boundPort = typeof address === "object" && address !== null ? address.port : port
				this.logger.info({ bind: `${host}:${boundPort}` }, `Admin UI available at http://${host}:${boundPort}/`)
				resolve(boundPort)
			})
		})
	}

	stop(): void {
		this.server.close()
	}

	private async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const path = (req.url ?? "/").split("?")[0]

		if (path === "/" || path === "/index.html") {
			if (req.method !== "GET") return this.json(res, 405, { error: "Method not allowed" })
			res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" })
			res.end(ADMIN_UI_HTML)
			return
		}

		if (path === "/health") {
			this.json(res, 200, { status: "ok" })
			return
		}

		if (path === "/api/strategies") {
			if (req.method !== "GET") return this.json(res, 405, { error: "Method not allowed" })
			this.json(res, 200, { strategies: this.strategies.map(serializeStrategy) })
			return
		}

		const curvesMatch = path.match(/^\/api\/strategies\/(\d+)\/curves$/)
		if (curvesMatch) {
			if (req.method !== "PUT") return this.json(res, 405, { error: "Method not allowed" })
			return this.handleCurveUpdate(req, res, Number(curvesMatch[1]))
		}

		this.json(res, 404, { error: "Not found" })
	}

	private async handleCurveUpdate(req: IncomingMessage, res: ServerResponse, index: number): Promise<void> {
		const strategy = this.strategies.find((s) => s.index === index)
		if (!strategy) {
			return this.json(res, 404, { error: `No strategy with index ${index}` })
		}

		let body: unknown
		try {
			body = JSON.parse(await this.readBody(req))
		} catch (err) {
			return this.json(res, 400, { error: err instanceof Error ? err.message : "Invalid JSON body" })
		}

		const shapeError = validateCurveUpdateShape(body)
		if (shapeError) {
			return this.json(res, 400, { error: shapeError })
		}
		const update = body as { bidPriceCurve?: PriceCurvePoint[]; askPriceCurve?: PriceCurvePoint[] }

		// A side without a policy is structurally uneditable: disabled (one-sided LP)
		// or venue-priced. Direction enablement is fixed at startup; this only moves prices.
		if (update.bidPriceCurve && !strategy.bid) {
			return this.json(res, 409, { error: "The bid side of this strategy is not editable (disabled or venue-priced)" })
		}
		if (update.askPriceCurve && !strategy.ask) {
			return this.json(res, 409, { error: "The ask side of this strategy is not editable (disabled or venue-priced)" })
		}

		// Apply all-or-nothing: validate every curve before touching any policy.
		const sides: Array<{ label: "bid" | "ask"; policy: FillerPricePolicy; points: PriceCurvePoint[] }> = []
		if (update.bidPriceCurve) sides.push({ label: "bid", policy: strategy.bid!, points: update.bidPriceCurve })
		if (update.askPriceCurve) sides.push({ label: "ask", policy: strategy.ask!, points: update.askPriceCurve })
		try {
			for (const side of sides) {
				// Constructing a throwaway policy runs full validation without mutating.
				void new FillerPricePolicy({ points: side.points })
			}
		} catch (err) {
			return this.json(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		for (const side of sides) {
			const previous = side.policy.getPoints()
			side.policy.replacePoints({ points: side.points })
			this.logger.info(
				{ strategy: index, side: side.label, previous, next: side.policy.getPoints() },
				"Price curve updated in memory (lost on restart)",
			)
		}

		this.json(res, 200, serializeStrategy(strategy))
	}

	private readBody(req: IncomingMessage): Promise<string> {
		return new Promise((resolve, reject) => {
			const chunks: Buffer[] = []
			let size = 0
			req.on("data", (chunk: Buffer) => {
				size += chunk.length
				if (size > MAX_BODY_BYTES) {
					reject(new Error("Request body too large"))
					req.destroy()
					return
				}
				chunks.push(chunk)
			})
			req.on("end", () => resolve(Buffer.concat(chunks).toString("utf-8")))
			req.on("error", reject)
		})
	}

	private json(res: ServerResponse, status: number, payload: unknown): void {
		res.writeHead(status, { "Content-Type": "application/json" })
		res.end(JSON.stringify(payload))
	}
}

function serializeStrategy(strategy: AdminStrategy) {
	return {
		index: strategy.index,
		exotic: strategy.exotic,
		pricingMode: strategy.bid || strategy.ask ? ("static" as const) : ("venue" as const),
		bid: strategy.bid?.getPoints(),
		ask: strategy.ask?.getPoints(),
	}
}

/** Returns an error message when the body is not a well-formed curve update, else null. */
function validateCurveUpdateShape(body: unknown): string | null {
	if (typeof body !== "object" || body === null || Array.isArray(body)) {
		return "Body must be a JSON object"
	}
	const { bidPriceCurve, askPriceCurve, ...rest } = body as Record<string, unknown>
	if (Object.keys(rest).length > 0) {
		return `Unknown fields: ${Object.keys(rest).join(", ")}`
	}
	if (bidPriceCurve === undefined && askPriceCurve === undefined) {
		return "Provide at least one of bidPriceCurve/askPriceCurve"
	}
	for (const [name, curve] of [
		["bidPriceCurve", bidPriceCurve],
		["askPriceCurve", askPriceCurve],
	] as const) {
		if (curve === undefined) continue
		if (!Array.isArray(curve) || curve.length === 0) {
			return `${name} must be a non-empty array of points`
		}
		for (const point of curve) {
			if (
				typeof point !== "object" ||
				point === null ||
				typeof (point as PriceCurvePoint).amount !== "string" ||
				typeof (point as PriceCurvePoint).price !== "string"
			) {
				return `Each ${name} point must have string 'amount' and 'price'`
			}
		}
	}
	return null
}
