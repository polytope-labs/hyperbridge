import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { writeConfigFileAtomic } from "@/config/write-config"
import { FillerPricePolicy, type PriceCurvePoint } from "@/config/interpolated-curve"
import type { FillerTomlConfig } from "@/config/filler-toml"
import { emitFillerToml } from "@/cli/init/emit-toml"
import { saveRuntimeState } from "@/core/runtime-state"
import { isAddress } from "viem"
import type { AllowlistConfig } from "@/services/FillerConfigService"
import type { ActivityLogService, ActivityEvent } from "../ActivityLogService"
import type { BalanceProvider } from "../BalanceProvider"
import type { BidStorageService } from "../BidStorageService"
import { configureLogger, getLogger, type LogLevel } from "../Logger"
import { readBody, sendJson, isLoopbackHost } from "./http-util"
import { serveStatic } from "./static"
import { handleSetupRequest, maskToml, type SetupDeps } from "./setup-api"

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

export type UiMode = "init" | "operator"

/** Narrow view of the IntentFiller, so tests can stub it. */
export interface PauseControl {
	pause(): void
	resume(): void
	isPaused(): boolean
	getWatchOnly(): Record<number, boolean>
}

/** Self-halt visibility/reset for one FX strategy (overfill protection). */
export interface HaltControl {
	index: number
	isHalted(): boolean
	resetHalt(): void
}

export interface OperatorContext {
	strategies: AdminStrategy[]
	filler: PauseControl
	balances: Pick<BalanceProvider, "getSnapshot">
	haltControls: HaltControl[]
	/** The running config; runtime edits (curves, allowlist, log level) are persisted back into it at configPath. */
	config: FillerTomlConfig
	/** Drains the filler and exits the process (the UI's graceful Stop). */
	stop(): Promise<void>
	activity: Pick<ActivityLogService, "getRecent" | "on" | "off">
	bids?: Pick<BidStorageService, "getRecentBids" | "getStats">
	vault?: { sweepNow(): Promise<void>; redeemAll(): Promise<void> }
	rebalancing?: { checkTriggers(): Promise<unknown> }
	/** Applies a new allowlist to the running filler (persistence handled by the server). */
	applyAllowlist(allowlist: AllowlistConfig | undefined): void
	version: string
	startedAt: number
	configPath: string
	chains: number[]
	strategyTypes: string[]
	dataDir?: string
}

export interface SetupContext {
	/** Default path the wizard writes the config to. */
	configPath: string
	/** Writes the config and boots the filler; the caller flips the server into operator mode. */
	onSaveAndStart(config: FillerTomlConfig, toml: string, path: string): Promise<void>
	/** Test injection for the network-facing validators. */
	deps?: SetupDeps
}

export type StartState = "idle" | "starting" | "running" | "failed"

const UI_NOT_BUILT_HTML = `<!doctype html><meta charset="utf-8"><title>simplex</title>
<body style="font-family:system-ui;margin:4rem auto;max-width:32rem">
<h1>UI not built</h1><p>The simplex web UI is missing from this build.
Run <code>pnpm ui:build</code> (or a full <code>pnpm build</code>) and restart.</p>
<p>The JSON API under <code>/api</code> is unaffected.</p></body>`

/**
 * Loopback HTTP server embedded in the simplex process. Serves the bundled SPA
 * and a JSON API in one of two modes: `init` (setup wizard endpoints, before a
 * config exists) or `operator` (status/pause/balances plus inflight price curve
 * updates on the running strategies). Unauthenticated: binding is the boundary —
 * init mode refuses non-loopback hosts outright.
 */
export class UiServer {
	private server: Server
	private logger = getLogger("ui")
	private mode: UiMode
	private operator?: OperatorContext
	private setup?: SetupContext
	private uiDistDir?: string
	private startState: StartState = "idle"
	private startError?: string
	private sseClients = new Set<ServerResponse>()
	private activityListener?: (event: ActivityEvent) => void

	constructor(opts: { mode: UiMode; uiDistDir?: string; setup?: SetupContext; operator?: OperatorContext }) {
		this.mode = opts.mode
		this.operator = opts.operator
		this.setup = opts.setup
		this.uiDistDir = opts.uiDistDir
		if (this.mode === "operator") this.startState = "running"
		if (this.operator) this.subscribeActivity()
		this.server = createServer((req, res) => {
			this.handle(req, res).catch((err) => {
				this.logger.error({ err }, "Unhandled UI request error")
				if (!res.headersSent) {
					sendJson(res, 500, { error: "Internal server error" })
				}
			})
		})
	}

	/** Resolves with the bound port once listening (pass port 0 for an ephemeral port). */
	start(port: number, host = "127.0.0.1"): Promise<number> {
		if (this.mode === "init" && !isLoopbackHost(host)) {
			return Promise.reject(
				new Error(`The setup wizard carries secrets and only binds loopback addresses, not ${host}`),
			)
		}
		if (this.mode === "operator" && !isLoopbackHost(host)) {
			this.logger.warn(
				{ host },
				"UI server binding a non-loopback address — it is unauthenticated, make sure the network is trusted",
			)
		}
		return new Promise((resolve, reject) => {
			this.server.once("error", reject)
			this.server.listen(port, host, () => {
				const address = this.server.address()
				const boundPort = typeof address === "object" && address !== null ? address.port : port
				this.logger.info({ bind: `${host}:${boundPort}` }, `Simplex UI available at http://${host}:${boundPort}/`)
				resolve(boundPort)
			})
		})
	}

	stop(): void {
		if (this.activityListener && this.operator) {
			this.operator.activity.off("event", this.activityListener)
			this.activityListener = undefined
		}
		for (const client of this.sseClients) client.end()
		this.sseClients.clear()
		this.server.close()
	}

	/** Flips a live init-mode server into operator mode; the listener keeps running. */
	enterOperatorMode(ctx: OperatorContext): void {
		this.operator = ctx
		this.mode = "operator"
		this.startState = "running"
		this.startError = undefined
		this.subscribeActivity()
		this.logger.info("Setup complete — UI now in operator mode")
	}

	/** Re-broadcasts activity rows to every open SSE connection. */
	private subscribeActivity(): void {
		if (this.activityListener || !this.operator) return
		this.activityListener = (event: ActivityEvent) => {
			const frame = `data: ${JSON.stringify(event)}\n\n`
			for (const client of this.sseClients) {
				client.write(frame)
			}
		}
		this.operator.activity.on("event", this.activityListener)
	}

	/** Reported by /api/setup/start-status while save-and-start boots the filler. */
	setStartState(state: StartState, error?: string): void {
		this.startState = state
		this.startError = error
	}

	getMode(): UiMode {
		return this.mode
	}

	getStartState(): StartState {
		return this.startState
	}

	private async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const path = (req.url ?? "/").split("?")[0]
		const method = req.method ?? "GET"

		// CSRF hygiene: a cross-origin page can't set this header without a
		// preflight, and no CORS headers are ever emitted.
		if (method !== "GET" && method !== "HEAD" && req.headers["x-simplex-ui"] !== "1") {
			return sendJson(res, 403, { error: "Missing X-Simplex-UI header" })
		}

		if (path === "/health") {
			return sendJson(res, 200, { status: "ok", mode: this.mode })
		}

		if (path === "/api/status") {
			return this.handleStatus(res)
		}

		if (path === "/api/setup/start-status") {
			return sendJson(res, 200, { state: this.startState, error: this.startError })
		}

		if (path.startsWith("/api/setup/")) {
			if (this.mode !== "init" || !this.setup) {
				return sendJson(res, 410, { error: "Setup already completed" })
			}
			return handleSetupRequest(this, this.setup, req, res, path, method)
		}

		if (path === "/api/strategies") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			return sendJson(res, 200, { strategies: this.operator!.strategies.map(serializeStrategy) })
		}

		const curvesMatch = path.match(/^\/api\/strategies\/(\d+)\/curves$/)
		if (curvesMatch) {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "PUT") return sendJson(res, 405, { error: "Method not allowed" })
			return this.handleCurveUpdate(req, res, Number(curvesMatch[1]))
		}

		if (path === "/api/pause" || path === "/api/resume") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
			const pause = path === "/api/pause"
			if (pause) this.operator!.filler.pause()
			else this.operator!.filler.resume()
			saveRuntimeState({ paused: pause }, this.operator!.dataDir)
			return sendJson(res, 200, { paused: this.operator!.filler.isPaused() })
		}

		if (path === "/api/balances") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			return sendJson(res, 200, this.operator!.balances.getSnapshot())
		}

		if (path === "/api/activity/orders") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const params = new URL(req.url ?? "/", "http://localhost").searchParams
			const limit = Number(params.get("limit") ?? 100)
			const before = params.get("before") ? Number(params.get("before")) : undefined
			return sendJson(res, 200, { events: this.operator!.activity.getRecent(limit, before) })
		}

		if (path === "/api/activity/bids") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const bids = this.operator!.bids
			if (!bids) return sendJson(res, 200, { bids: [], stats: null })
			const params = new URL(req.url ?? "/", "http://localhost").searchParams
			return sendJson(res, 200, {
				bids: bids.getRecentBids(Number(params.get("limit") ?? 100)),
				stats: bids.getStats(),
			})
		}

		if (path === "/api/events") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			res.writeHead(200, {
				"Content-Type": "text/event-stream",
				"Cache-Control": "no-store",
				Connection: "keep-alive",
			})
			res.write(":ok\n\n")
			this.sseClients.add(res)
			req.on("close", () => this.sseClients.delete(res))
			return
		}

		if (path === "/api/config") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const op = this.operator!
			return sendJson(res, 200, {
				configPath: op.configPath,
				toml: maskToml(op.config),
				logLevel: op.config.simplex.logging ?? "info",
				vaultConfigured: Boolean(op.vault),
			})
		}

		if (path === "/api/log-level") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "PUT") return sendJson(res, 405, { error: "Method not allowed" })
			return this.handleLogLevel(req, res)
		}

		if (path === "/api/allowlist") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "PUT") return sendJson(res, 405, { error: "Method not allowed" })
			return this.handleAllowlist(req, res)
		}

		if (path === "/api/vault/sweep" || path === "/api/vault/redeem") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
			const vault = this.operator!.vault
			if (!vault) return sendJson(res, 409, { error: "No vault configured" })
			try {
				if (path === "/api/vault/sweep") await vault.sweepNow()
				else await vault.redeemAll()
				return sendJson(res, 200, { ok: true })
			} catch (err) {
				return sendJson(res, 500, { error: err instanceof Error ? err.message : String(err) })
			}
		}

		if (path === "/api/rebalancing") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const op = this.operator!
			if (!op.rebalancing || !op.config.rebalancing) {
				return sendJson(res, 200, { configured: false })
			}
			try {
				return sendJson(res, 200, {
					configured: true,
					triggerPercentage: op.config.rebalancing.triggerPercentage,
					baseBalances: op.config.rebalancing.baseBalances,
					triggers: await op.rebalancing.checkTriggers(),
				})
			} catch (err) {
				return sendJson(res, 500, { error: err instanceof Error ? err.message : String(err) })
			}
		}

		if (path === "/api/reset-halt") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
			for (const control of this.operator!.haltControls) control.resetHalt()
			return sendJson(res, 200, { halted: [] })
		}

		if (path === "/api/stop") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
			this.logger.warn("Graceful stop requested from the UI")
			sendJson(res, 202, { stopping: true })
			// Let the response flush before draining the filler and exiting.
			setTimeout(() => void this.operator!.stop(), 100)
			return
		}

		if (path.startsWith("/api/")) {
			return sendJson(res, 404, { error: "Not found" })
		}

		if (method !== "GET" && method !== "HEAD") {
			return sendJson(res, 405, { error: "Method not allowed" })
		}
		if (this.uiDistDir && serveStatic(res, this.uiDistDir, path)) return
		if (path === "/" || path === "/index.html") {
			res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" })
			res.end(UI_NOT_BUILT_HTML)
			return
		}
		res.writeHead(404, { "Content-Type": "text/plain" })
		res.end("Not found")
	}

	private handleStatus(res: ServerResponse): void {
		if (this.mode === "init" || !this.operator) {
			return sendJson(res, 200, {
				mode: "init",
				starting: this.startState === "starting",
				startError: this.startError,
			})
		}
		const op = this.operator
		return sendJson(res, 200, {
			mode: "operator",
			version: op.version,
			uptimeSec: Math.floor((Date.now() - op.startedAt) / 1000),
			paused: op.filler.isPaused(),
			halted: op.haltControls.filter((h) => h.isHalted()).map((h) => h.index),
			watchOnly: op.filler.getWatchOnly(),
			chains: op.chains,
			strategies: op.strategies.map((s) => ({ index: s.index, exotic: s.exotic })),
			strategyTypes: op.strategyTypes,
			configPath: op.configPath,
		})
	}

	private async handleCurveUpdate(req: IncomingMessage, res: ServerResponse, index: number): Promise<void> {
		const strategy = this.operator!.strategies.find((s) => s.index === index)
		if (!strategy) {
			return sendJson(res, 404, { error: `No strategy with index ${index}` })
		}

		let body: unknown
		try {
			body = JSON.parse(await readBody(req))
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : "Invalid JSON body" })
		}

		const shapeError = validateCurveUpdateShape(body)
		if (shapeError) {
			return sendJson(res, 400, { error: shapeError })
		}
		const update = body as { bidPriceCurve?: PriceCurvePoint[]; askPriceCurve?: PriceCurvePoint[] }

		// A side without a policy is structurally uneditable: disabled (one-sided LP)
		// or venue-priced. Direction enablement is fixed at startup; this only moves prices.
		if (update.bidPriceCurve && !strategy.bid) {
			return sendJson(res, 409, { error: "The bid side of this strategy is not editable (disabled or venue-priced)" })
		}
		if (update.askPriceCurve && !strategy.ask) {
			return sendJson(res, 409, { error: "The ask side of this strategy is not editable (disabled or venue-priced)" })
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
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		for (const side of sides) {
			const previous = side.policy.getPoints()
			side.policy.replacePoints({ points: side.points })
			this.logger.info(
				{ strategy: index, side: side.label, previous, next: side.policy.getPoints() },
				"Price curve updated on the running strategy",
			)
		}

		const persisted = this.persistCurveUpdate(index, update)
		sendJson(res, 200, { ...serializeStrategy(strategy), persisted })
	}

	/**
	 * Writes the updated curves back into the config file so restarts keep them.
	 * The file is regenerated from the parsed config: hand-written comments are
	 * replaced by the generated ones, values are preserved.
	 */
	private persistCurveUpdate(
		index: number,
		update: { bidPriceCurve?: PriceCurvePoint[]; askPriceCurve?: PriceCurvePoint[] },
	): boolean {
		const op = this.operator!
		const strategy = op.config.strategies[index]
		if (!strategy || strategy.type !== "hyperfx") return false
		if (update.bidPriceCurve) strategy.bidPriceCurve = update.bidPriceCurve
		if (update.askPriceCurve) strategy.askPriceCurve = update.askPriceCurve
		return this.persistConfig()
	}

	/** Regenerates the config file from the (mutated) running config. */
	private persistConfig(): boolean {
		const op = this.operator!
		try {
			writeConfigFileAtomic(op.configPath, emitFillerToml(op.config))
			return true
		} catch (err) {
			this.logger.warn({ err, configPath: op.configPath }, "Change applied in memory but could not be persisted")
			return false
		}
	}

	private async handleLogLevel(req: IncomingMessage, res: ServerResponse): Promise<void> {
		let body: { level?: string }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		const level = body.level
		if (!level || !["trace", "debug", "info", "warn", "error"].includes(level)) {
			return sendJson(res, 400, { error: "level must be one of trace, debug, info, warn, error" })
		}
		configureLogger(level as LogLevel)
		this.operator!.config.simplex.logging = level
		const persisted = this.persistConfig()
		this.logger.warn({ level }, "Log level changed from the UI")
		return sendJson(res, 200, { level, persisted })
	}

	private async handleAllowlist(req: IncomingMessage, res: ServerResponse): Promise<void> {
		let body: { users?: string[] }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (!Array.isArray(body.users)) {
			return sendJson(res, 400, { error: "Provide users as an array (empty to accept all users)" })
		}
		const users = body.users.map((u) => String(u).trim()).filter(Boolean)
		const invalid = users.find((u) => !isAddress(u))
		if (invalid) {
			return sendJson(res, 400, { error: `Invalid address: ${invalid}` })
		}

		const op = this.operator!
		// An empty list means "no allowlist" (accept everyone) — a present-but-empty
		// allowlist would reject every order.
		const bySource = op.config.allowlist?.bySource
		const allowlist: AllowlistConfig | undefined =
			users.length > 0 || (bySource && Object.keys(bySource).length > 0)
				? { ...(users.length > 0 ? { users } : {}), ...(bySource ? { bySource } : {}) }
				: undefined
		op.applyAllowlist(allowlist)
		op.config.allowlist = allowlist
		const persisted = this.persistConfig()
		this.logger.warn({ userCount: users.length }, "Allowlist updated from the UI")
		return sendJson(res, 200, { users, persisted })
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
