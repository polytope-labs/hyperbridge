import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { Decimal } from "decimal.js"
import { writeConfigFileAtomic } from "@/config/write-config"
import { FillerPricePolicy, formatChainKey, parseChainKey, type PriceCurvePoint } from "@/config/interpolated-curve"
import { AssetRegistry, registrySymbols } from "@/config/asset-registry"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { INIT_CHAINS } from "@/cli/init/chains"
import { ChainConfigService } from "@hyperbridge/sdk"
import type { FillerTomlConfig, VaultToml } from "@/config/filler-toml"
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
import {
	LOG_LEVELS,
	type AdminStrategyDto,
	type ConfigDto,
	type SendTokenOption,
	type StatusInit,
	type StatusOperator,
	type WalletTxDto,
} from "./dto"

/**
 * One curve-priced trading pair's editable price curves. The policies are the
 * same instances the running engine prices with, so `replacePoints` takes
 * effect on the next order evaluation. A side is absent when it cannot be
 * edited: disabled (one-sided LP) or venue-priced (both sides absent).
 */
export interface AdminStrategy {
	/** Position among curve-priced pairs; stable identifier for the API. */
	index: number
	/** Position in the TOML `[[pairs]]` array — where curve edits are persisted. */
	pairIndex: number
	/** Pair label, e.g. "USDC/CNGN" (display-only). */
	exotic?: string
	token0: string
	token1: string
	bid?: FillerPricePolicy
	ask?: FillerPricePolicy
	/** Same-asset cross-chain market: ask-only, prices strictly below par. */
	sameToken?: boolean
	/** Price feed only — the pair never fills; curves stay editable, sides are never opened. */
	referenceOnly?: boolean
	/**
	 * Opens a direction configured as one-sided LP with a fresh policy. Present
	 * only for cross-asset curve-priced pairs — same-token markets stay
	 * ask-only and venue-priced sides stay uneditable.
	 */
	enableSide?: (side: "bid" | "ask", policy: FillerPricePolicy) => void
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
	activity: Pick<ActivityLogService, "getRecent" | "on" | "off" | "recordWalletTx" | "getWalletTxs" | "getFillTxs">
	bids?: Pick<BidStorageService, "getRecentBids" | "getStats">
	vault?: {
		sweepNow(): Promise<void>
		redeemAll(): Promise<void>
		/** Re-hydrates the shared venue with a new vault set; rejects on bad vaults. */
		reconfigure(vaults: VaultToml[], sweepIntervalMs?: number): Promise<void>
	}
	rebalancing?: { checkTriggers(): Promise<unknown> }
	/** Applies a new allowlist to the running filler (persistence handled by the server). */
	applyAllowlist(allowlist: AllowlistConfig | undefined): void
	/** Applies rebalancing settings to the running filler; trigger checks read them live. */
	applyRebalancing(rebalancing: FillerTomlConfig["rebalancing"]): void
	/**
	 * Hydration-level validation of a prospective vault set (unconfigured chain,
	 * same-asset duplicates, non-vault address) without touching any live venue.
	 */
	vaultPreflight?: (vaults: VaultToml[]) => Promise<void>
	/** Outbound transfer from the filler wallet; vault shares are redeemed to the recipient. */
	send?: (params: { chain: string; token: string; amount: string; to: `0x${string}` }) => Promise<{
		txHash: string
		sponsored: boolean
		redeemed: boolean
	}>
	version: string
	startedAt: number
	configPath: string
	chains: number[]
	strategyTypes: string[]
	/** Filler accounts, shown permanently on the dashboard for funding. */
	addresses?: { evm: string; substrate?: string }
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

		if (path === "/api/wallet/history") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const params = new URL(req.url ?? "/", "http://localhost").searchParams
			const limit = Math.min(Math.max(Number(params.get("limit") ?? 100), 1), 500)
			const activity = this.operator!.activity
			const txs: WalletTxDto[] = [
				...activity.getWalletTxs(limit).map((tx) => ({ ...tx, id: `wallet-${tx.id}` })),
				...activity.getFillTxs(limit).map((event) => ({
					id: `fill-${event.id}`,
					ts: event.ts,
					kind: "fill" as const,
					chainId: event.chainId,
					token: null,
					amount: null,
					to: null,
					txHash: event.txHash as string,
					sponsored: null,
				})),
			]
				.sort((a, b) => b.ts - a.ts)
				.slice(0, limit)
			return sendJson(res, 200, { txs })
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
			const configDto: ConfigDto = {
				configPath: op.configPath,
				toml: maskToml(op.config),
				logLevel: op.config.simplex.logging ?? "info",
				vaultConfigured: Boolean(op.vault),
				allowlistUsers: op.config.allowlist?.users ?? [],
				vaults: op.config.vault?.vaults ?? [],
				sendTokens: this.sendTokenOptions(op),
			}
			return sendJson(res, 200, configDto)
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

		if (path === "/api/send" && method === "POST") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			return this.handleSend(req, res)
		}

		if (path === "/api/vault" && method === "PUT") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			return this.handleVaultUpdate(req, res)
		}

		if (path === "/api/rebalancing" && method === "PUT") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			return this.handleRebalancingUpdate(req, res)
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
			const status: StatusInit = {
				mode: "init",
				starting: this.startState === "starting",
				startError: this.startError,
			}
			return sendJson(res, 200, status)
		}
		const op = this.operator
		const status: StatusOperator = {
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
			addresses: op.addresses,
			chainLabels: Object.fromEntries(
				op.chains.map((id) => [id, INIT_CHAINS.find((c) => c.chainId === id)?.label ?? `chain ${id}`]),
			),
		}
		return sendJson(res, 200, status)
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

		// A curve on a disabled side of a curve-priced pair *enables* that side
		// (one-sided LP opened by the operator). Venue-priced pairs expose no
		// enableSide, and same-token markets are ask-only by construction.
		const enabling: Array<{ side: "bid" | "ask"; points: PriceCurvePoint[] }> = []
		if (update.bidPriceCurve && !strategy.bid) {
			if (!strategy.enableSide) {
				return sendJson(res, 409, {
					error: strategy.sameToken
						? "Same-token markets are ask-only — the bid side cannot be enabled"
						: "The bid side of this strategy is not editable (venue-priced)",
				})
			}
			enabling.push({ side: "bid", points: update.bidPriceCurve })
		}
		if (update.askPriceCurve && !strategy.ask) {
			if (!strategy.enableSide) {
				return sendJson(res, 409, { error: "The ask side of this strategy is not editable (venue-priced)" })
			}
			enabling.push({ side: "ask", points: update.askPriceCurve })
		}

		// Apply all-or-nothing: validate every curve before touching any policy.
		const sides: Array<{ label: "bid" | "ask"; policy: FillerPricePolicy; points: PriceCurvePoint[] }> = []
		if (update.bidPriceCurve && strategy.bid) sides.push({ label: "bid", policy: strategy.bid, points: update.bidPriceCurve })
		if (update.askPriceCurve && strategy.ask) sides.push({ label: "ask", policy: strategy.ask, points: update.askPriceCurve })
		const enabled: Array<{ side: "bid" | "ask"; policy: FillerPricePolicy }> = []
		try {
			for (const side of sides) {
				// Constructing a throwaway policy runs full validation without mutating.
				void new FillerPricePolicy({ points: side.points })
			}
			for (const enable of enabling) {
				enabled.push({ side: enable.side, policy: new FillerPricePolicy({ points: enable.points }) })
			}

			// Live edits keep the same book invariants as startup. Judge the
			// proposed side against the untouched other side, so a one-sided
			// edit cannot cross the book or push a same-token ask to par.
			const nextBid = update.bidPriceCurve ? new FillerPricePolicy({ points: update.bidPriceCurve }) : strategy.bid
			const nextAsk = update.askPriceCurve ? new FillerPricePolicy({ points: update.askPriceCurve }) : strategy.ask
			if (strategy.sameToken && nextAsk) {
				for (const point of nextAsk.getPoints()) {
					if (new Decimal(point.price).gte(1)) {
						throw new Error(
							`same-token ask prices must be strictly below 1 — '${point.price}' would fill at or above par`,
						)
					}
				}
			}
			if (!strategy.sameToken && nextBid && nextAsk) {
				FillerPricePolicy.assertBookNotCrossed(`strategy ${index}`, nextBid, nextAsk)
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
		for (const { side, policy } of enabled) {
			strategy.enableSide!(side, policy)
			if (side === "bid") strategy.bid = policy
			else strategy.ask = policy
			this.logger.warn(
				{ strategy: index, side, points: policy.getPoints() },
				"One-sided LP direction enabled from the UI",
			)
		}

		const persisted = this.persistCurveUpdate(strategy, update)
		sendJson(res, 200, { ...serializeStrategy(strategy), persisted })
	}

	/**
	 * Writes the updated curves back into the config file so restarts keep them.
	 * The file is regenerated from the parsed config: hand-written comments are
	 * replaced by the generated ones, values are preserved.
	 */
	private persistCurveUpdate(
		strategy: AdminStrategy,
		update: { bidPriceCurve?: PriceCurvePoint[]; askPriceCurve?: PriceCurvePoint[] },
	): boolean {
		const op = this.operator!
		const pair = op.config.pairs?.[strategy.pairIndex]
		if (!pair) return false
		if (update.bidPriceCurve) pair.bidPriceCurve = update.bidPriceCurve
		if (update.askPriceCurve) pair.askPriceCurve = update.askPriceCurve
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
		if (!level || !(LOG_LEVELS as readonly string[]).includes(level)) {
			return sendJson(res, 400, { error: `level must be one of ${LOG_LEVELS.join(", ")}` })
		}
		configureLogger(level as LogLevel)
		this.operator!.config.simplex.logging = level
		const persisted = this.persistConfig()
		this.logger.warn({ level }, "Log level changed from the UI")
		return sendJson(res, 200, { level, persisted })
	}

	/**
	 * Token choices for the dashboard Send card, per state machine id: native
	 * plus the chain's stablecoins/exotics from the SDK registry. Vault shares
	 * are not listed — sends of the underlying draw on the vault when the
	 * wallet balance falls short.
	 */
	private sendTokenOptions(op: OperatorContext): Record<string, SendTokenOption[]> {
		// The same symbol registry the trading engine resolves pairs with:
		// built-ins from the SDK chain registry plus the config's [assets] table.
		const registry = new AssetRegistry(new ChainConfigService({}), op.config.assets)
		const symbols = [
			...registrySymbols(),
			...Object.keys(op.config.assets ?? {}).filter((s) => !registrySymbols().includes(s.trim().toUpperCase())),
		]
		const options: Record<string, SendTokenOption[]> = {}
		for (const chainId of op.chains) {
			const stateMachineId = formatChainKey(chainId)
			const tokens: SendTokenOption[] = [{ symbol: "native", address: "native" }]
			for (const symbol of symbols) {
				const address = registry.getAddress(symbol, stateMachineId)
				if (address) tokens.push({ symbol, address })
			}
			options[stateMachineId] = tokens
		}
		return options
	}

	private async handleSend(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const op = this.operator!
		if (!op.send) return sendJson(res, 501, { error: "Sending is not available on this filler" })
		let body: { chain?: string; token?: string; amount?: string; to?: string }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (!body.chain || !body.token || !body.amount || !body.to) {
			return sendJson(res, 400, { error: "chain, token, amount and to are required" })
		}
		if (!/^0x[0-9a-fA-F]{40}$/.test(body.to)) {
			return sendJson(res, 400, { error: `Invalid recipient address: ${body.to}` })
		}
		if (!(Number(body.amount) > 0)) {
			return sendJson(res, 400, { error: `Invalid amount: ${body.amount}` })
		}
		try {
			const result = await op.send({
				chain: body.chain,
				token: body.token,
				amount: body.amount,
				to: body.to as `0x${string}`,
			})
			this.logger.warn({ ...body, ...result }, "Operator send submitted from the UI")
			const symbol =
				(this.sendTokenOptions(op)[body.chain] ?? []).find(
					(option) => option.address.toLowerCase() === body.token!.toLowerCase(),
				)?.symbol ?? body.token
			try {
				op.activity.recordWalletTx({
					kind: "send",
					chainId: parseChainKey(body.chain),
					token: symbol,
					amount: body.amount,
					to: body.to,
					txHash: result.txHash,
					sponsored: result.sponsored,
				})
			} catch (err) {
				this.logger.warn({ err }, "Failed to record send in wallet history")
			}
			return sendJson(res, 200, result)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}
	}

	private async handleVaultUpdate(req: IncomingMessage, res: ServerResponse): Promise<void> {
		let body: { vaults?: VaultToml[]; sweepIntervalMs?: number }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (!Array.isArray(body.vaults)) {
			return sendJson(res, 400, { error: "Provide vaults as an array (empty to disable sourcing/sweeping)" })
		}
		try {
			VaultFundingPlanner.validateConfig(body.vaults)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		const op = this.operator!
		// A venue only exists when the boot config had one; enabling from nothing
		// needs strategy re-wiring, which is a restart. Hydration-level errors
		// (same-asset duplicates, unconfigured chain, non-vault address) must
		// still reject here — persisting them would brick the next boot.
		if (!op.vault) {
			try {
				await op.vaultPreflight?.(body.vaults)
			} catch (err) {
				return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
			}
			op.config.vault = { vaults: body.vaults, ...(body.sweepIntervalMs ? { sweepIntervalMs: body.sweepIntervalMs } : {}) }
			const persisted = this.persistConfig()
			return sendJson(res, 200, { applied: false, restartNeeded: true, persisted })
		}

		try {
			// Bad vault addresses reject here (hydration reads asset() on-chain),
			// leaving the previous set live.
			await op.vault.reconfigure(body.vaults, body.sweepIntervalMs)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}
		op.config.vault =
			body.vaults.length > 0
				? { vaults: body.vaults, ...(body.sweepIntervalMs ? { sweepIntervalMs: body.sweepIntervalMs } : {}) }
				: undefined
		const persisted = this.persistConfig()
		this.logger.warn({ vaultCount: body.vaults.length }, "Vault treasury updated from the UI")
		return sendJson(res, 200, { applied: true, restartNeeded: false, persisted })
	}

	private async handleRebalancingUpdate(req: IncomingMessage, res: ServerResponse): Promise<void> {
		let body: {
			triggerPercentage?: number
			baseBalances?: { USDC?: Record<string, string>; USDT?: Record<string, string> }
		}
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		const trigger = Number(body.triggerPercentage)
		if (!Number.isFinite(trigger) || trigger <= 0 || trigger >= 1) {
			return sendJson(res, 400, { error: "triggerPercentage must be between 0 and 1 (exclusive)" })
		}
		const baseBalances = body.baseBalances ?? {}
		const entries = [...Object.entries(baseBalances.USDC ?? {}), ...Object.entries(baseBalances.USDT ?? {})]
		if (entries.length === 0) {
			return sendJson(res, 400, { error: "Provide at least one base balance" })
		}
		for (const [chainId, amount] of entries) {
			if (!/^\d+$/.test(chainId) || !(Number(amount) > 0)) {
				return sendJson(res, 400, { error: `Invalid base balance for chain ${chainId}: ${amount}` })
			}
		}

		const op = this.operator!
		const rebalancing = { triggerPercentage: trigger, baseBalances }
		op.applyRebalancing(rebalancing)
		op.config.rebalancing = rebalancing
		const persisted = this.persistConfig()
		// The trigger loop only runs when rebalancing was configured at boot.
		const applied = Boolean(op.rebalancing)
		this.logger.warn({ trigger, chains: entries.length }, "Rebalancing settings updated from the UI")
		return sendJson(res, 200, { applied, restartNeeded: !applied, persisted })
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

function serializeStrategy(strategy: AdminStrategy): AdminStrategyDto {
	return {
		index: strategy.index,
		exotic: strategy.exotic,
		token0: strategy.token0,
		token1: strategy.token1,
		pricingMode: strategy.bid || strategy.ask ? ("static" as const) : ("venue" as const),
		sameToken: strategy.sameToken ?? false,
		referenceOnly: strategy.referenceOnly ?? false,
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
