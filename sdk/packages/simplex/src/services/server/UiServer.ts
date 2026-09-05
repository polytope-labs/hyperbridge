import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { Decimal } from "decimal.js"
import { FillerPricePolicy, formatChainKey, parseChainKey, type PriceCurvePoint } from "@/config/interpolated-curve"
import { AssetRegistry, registrySymbols, validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
import { assertPairSymbolsResolve, validatePairConfigs, type PairConfig } from "@/config/pairs"
import { VaultFundingPlanner, type VaultSweepResult } from "@/funding/vault/VaultFundingPlanner"
import { chainByChainId, chainsForNetwork, INIT_CHAINS, type InitNetwork } from "@/cli/init/chains"
import { TESTNET_CONFIRMATION_POINTS } from "@/cli/init/state"
import { ChainConfigService } from "@hyperbridge/sdk"
import { assertConfirmationCoverage, type FillerConfigFile, type FillerTomlConfig, type VaultToml } from "@/config/filler-toml"
import { emitFillerToml, writeConfigFileAtomic } from "@/cli/init/emit-toml"
import { formatUnits, isAddress } from "viem"
import { validateRpcUrls, type AllowlistConfig } from "@/services/FillerConfigService"
import { withTimeout, PROBE_TIMEOUT_MS } from "@/cli/init/prompt-utils"
import type { ActivityRecorder } from "@/data/recorder"
import type { ActivityEvent, BidStore } from "@/data/types"
import type { BalanceProvider } from "../BalanceProvider"
import { getLogger, type LogLevel } from "../Logger"
import { readBody, sendJson, isLoopbackHost, isContainerized, hostHeaderAllowed } from "./http-util"
import { serveStatic } from "./static"
import {
	handleSetupRequest,
	maskToml,
	resolveSetupDeps,
	validateAlchemyKey,
	validateBundler,
	validateRpc,
	validateToken,
	type SetupDeps,
} from "./setup-api"
import {
	LOG_LEVELS,
	type AdminStrategyDto,
	type ChainRowDto,
	type ChainsDto,
	type ConfigDto,
	type SendTokenOption,
	type StatusInit,
	type StatusOperator,
	type WalletTxDto,
	type VaultSweepDto,
	type OrderHistoryDto,
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
	/**
	 * Per-order cap in token0 units, as configured; absent when the market is
	 * uncapped. Kept in step with `setMaxOrderSize` and `clearMaxOrderSize`.
	 */
	maxOrderSize?: string
	/**
	 * Applies a new per-order cap to the live TradingPair — the engine reads it
	 * per order, so the cap binds on the next evaluation. Absent when no engine
	 * ran at boot (the edit is then persisted for the next start).
	 */
	setMaxOrderSize?: (value: string) => void
	/**
	 * Removes the per-order cap from the live TradingPair, leaving the market
	 * uncapped — it then fills every order at its full notional. Same
	 * availability as `setMaxOrderSize`.
	 */
	clearMaxOrderSize?: () => void
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
	/** Closes a direction (back to one-sided LP); same availability as enableSide. */
	disableSide?: (side: "bid" | "ask") => void
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
	/**
	 * The running config. Typed as the file shape because `persistConfig`
	 * regenerates the TOML from it: a `[simplex.signer]` block the binary parsed
	 * rides along untouched, and dropping it here would erase the operator's
	 * signer from their config file on the next curve edit.
	 */
	config: FillerConfigFile
	/** Drains the filler and exits the process (the UI's graceful Stop). */
	stop(): Promise<void>
	activity: Pick<ActivityRecorder, "recent" | "on" | "off" | "record" | "recordWalletTx" | "walletTxs" | "fills" | "orderHistory">
	bids?: Pick<BidStore, "recent" | "stats" | "byCommitments">
	/** Persists an operator pause so it survives a restart. */
	setPaused(paused: boolean): Promise<void>
	/**
	 * Sets the running filler's verbosity. Logging is scoped per filler, so
	 * `configureLogger` alone moves only the process-wide fallback context and
	 * leaves this filler's own output untouched.
	 */
	setLogLevel(level: LogLevel): void
	vault?: {
		/** Runs one sweep pass now and reports what it did — and, per vault, why it did nothing. */
		sweepNow(): Promise<VaultSweepResult>
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
	 * Hydrates a validated new market into the running engine and returns its
	 * editable-curve view (null for venue-priced pairs). `assets` registers new
	 * custom tokens on the live registry first. Absent when no engine ran at boot.
	 */
	addPair?: (
		pair: PairConfig,
		assets: Record<string, AssetDefinition> | undefined,
		pairIndex: number,
	) => Promise<AdminStrategy | null>
	/** Removes a live market by strategy index; later strategies' pairIndex shift down with config.pairs. */
	removePair?: (index: number) => Promise<void>
	/** First RPC URL for a running chain (state machine id) — backs the custom-token verify probe. */
	rpcUrlFor?: (chain: string) => string | undefined
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
	/** Where runtime config edits are written back. Absent for a config-object filler. */
	configPath?: string
	chains: number[]
	strategyTypes: string[]
	/** Filler accounts, shown permanently on the dashboard for funding. */
	addresses?: { evm: string; substrate?: string }
}

export interface SetupContext {
	/** Default path the wizard writes the config to. */
	configPath: string
	/** Writes the config and boots the filler; the caller flips the server into operator mode. */
	onSaveAndStart(config: FillerConfigFile, toml: string, path: string): Promise<void>
	/** Test injection for the network-facing validators. */
	deps?: SetupDeps
}

export type StartState = "idle" | "starting" | "running" | "failed"

/** Setup routes that stay open in operator mode: stateless probes with no wizard state. */
const OPERATOR_PROBES = [
	"/api/setup/validate-token",
	"/api/setup/validate-rpc",
	"/api/setup/validate-bundler",
	"/api/setup/validate-alchemy-key",
]

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
	private boundLoopback = true
	private deps: Required<SetupDeps>
	/**
	 * Chain ids aligned with `config.chains` rows. The TOML records no chain id
	 * — boot derives it from each row's RPC — so the running set supplies the
	 * mapping until the operator edits the chain list, after which the edited
	 * ids stand in (the file is authoritative again on the next boot).
	 */
	private configuredChainIds?: number[]

	constructor(opts: {
		mode: UiMode
		uiDistDir?: string
		setup?: SetupContext
		operator?: OperatorContext
		/** Test injection for the operator-mode network probes (chain editor, token verify). */
		deps?: SetupDeps
	}) {
		this.mode = opts.mode
		this.operator = opts.operator
		this.setup = opts.setup
		this.uiDistDir = opts.uiDistDir
		this.deps = resolveSetupDeps(opts.deps)
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
			// Outside a container the host's interfaces are the real ones, and the wizard
			// collects private keys — the bind is refused. Inside one, see isContainerized().
			if (!isContainerized()) {
				return Promise.reject(
					new Error(`The setup wizard carries secrets and only binds loopback addresses, not ${host}`),
				)
			}
			this.logger.warn(
				{ host },
				"Setup wizard bound a non-loopback address inside a container — it collects private keys, so publish its port to 127.0.0.1 only",
			)
		}
		if (this.mode === "operator" && !isLoopbackHost(host)) {
			this.logger.warn(
				{ host },
				"UI server binding a non-loopback address — it is unauthenticated, make sure the network is trusted",
			)
		}
		this.boundLoopback = isLoopbackHost(host)
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

		// DNS-rebinding defense: an attacker page resolving its own domain to
		// this address becomes same-origin and could drive every endpoint,
		// including /api/send. A rebound origin always carries its DNS name in
		// Host, so only IP-literal/localhost Hosts are served.
		if (!hostHeaderAllowed(req.headers.host, this.boundLoopback)) {
			return sendJson(res, 403, { error: "Host header is not allowed" })
		}

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
			// Stateless probes, also needed by the operator forms (custom-token
			// verify, chain editor) — the only setup routes usable after boot.
			if (OPERATOR_PROBES.includes(path) && this.mode === "operator") {
				if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
				try {
					const body = JSON.parse(await readBody(req)) as Record<string, unknown>
					if (path === "/api/setup/validate-rpc") return sendJson(res, 200, await validateRpc(body, this.deps))
					if (path === "/api/setup/validate-bundler") {
						return sendJson(res, 200, await validateBundler(body, this.deps))
					}
					if (path === "/api/setup/validate-alchemy-key") {
						return sendJson(res, 200, await validateAlchemyKey(body, this.deps))
					}
					// The operator form sends a chain key, not an RPC URL — the
					// running config's endpoint for that chain backs the probe.
					if (!body.rpcUrl && typeof body.chain === "string") {
						body.rpcUrl = this.operator!.rpcUrlFor?.(body.chain)
					}
					return sendJson(res, 200, await validateToken(body))
				} catch (err) {
					return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
				}
			}
			if (this.mode !== "init" || !this.setup) {
				return sendJson(res, 410, { error: "Setup already completed" })
			}
			return handleSetupRequest(this, this.setup, req, res, path, method)
		}

		if (path === "/api/strategies") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method === "GET") {
				return sendJson(res, 200, { strategies: this.operator!.strategies.map(serializeStrategy) })
			}
			if (method === "POST") return this.handleMarketAdd(req, res)
			return sendJson(res, 405, { error: "Method not allowed" })
		}

		const strategyMatch = path.match(/^\/api\/strategies\/(\d+)$/)
		if (strategyMatch) {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method === "DELETE") return this.handleMarketRemove(res, Number(strategyMatch[1]))
			if (method === "PUT") return this.handleMarketUpdate(req, res, Number(strategyMatch[1]))
			return sendJson(res, 405, { error: "Method not allowed" })
		}

		const capMatch = path.match(/^\/api\/strategies\/(\d+)\/max-order-size$/)
		if (capMatch) {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "DELETE") return sendJson(res, 405, { error: "Method not allowed" })
			return this.handleMaxOrderSizeClear(res, Number(capMatch[1]))
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
			await this.operator!.setPaused(pause)
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
			return sendJson(res, 200, { events: await this.operator!.activity.recent(limit, before) })
		}

		if (path === "/api/activity/history") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const params = new URL(req.url ?? "/", "http://localhost").searchParams
			const page = Math.max(1, Number(params.get("page") ?? 1) || 1)
			const pageSize = Math.min(Math.max(Number(params.get("pageSize") ?? 20) || 20, 1), 100)
			const activity = this.operator!.activity
			const [history, newest] = await Promise.all([activity.orderHistory(page, pageSize), activity.recent(100)])
			const commitments = history.orders.map((order) => order.orderId)
			const bids = this.operator!.bids ? await this.operator!.bids.byCommitments(commitments) : []
			const bidsByOrder = new Map<string, typeof bids>()
			for (const bid of bids) {
				const list = bidsByOrder.get(bid.commitment) ?? []
				list.push(bid)
				bidsByOrder.set(bid.commitment, list)
			}
			const dto: OrderHistoryDto = {
				page: history.page,
				pageSize: history.pageSize,
				total: history.total,
				network: runningNetwork(this.operator!.chains),
				orders: history.orders.map((order) => ({ ...order, bids: bidsByOrder.get(order.orderId) ?? [] })),
				other: newest.filter((event) => event.orderId === null),
			}
			return sendJson(res, 200, dto)
		}

		if (path === "/api/wallet/history") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const params = new URL(req.url ?? "/", "http://localhost").searchParams
			const limit = Math.min(Math.max(Number(params.get("limit") ?? 100), 1), 500)
			const activity = this.operator!.activity
			const [walletTxs, fillTxs] = await Promise.all([activity.walletTxs(limit), activity.fills(limit)])
			const txs: WalletTxDto[] = [
				...walletTxs.map((tx) => ({ ...tx, id: `wallet-${tx.id}` })),
				...fillTxs.map((event) => ({
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
			const [recentBids, stats] = await Promise.all([
				bids.recent(Number(params.get("limit") ?? 100)),
				bids.stats(),
			])
			return sendJson(res, 200, { bids: recentBids, stats })
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
				knownVaults: this.knownVaultCatalog(op),
			}
			return sendJson(res, 200, configDto)
		}

		if (path === "/api/chains") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method === "GET") return this.handleChainsGet(res)
			if (method === "PUT") return this.handleChainsUpdate(req, res)
			return sendJson(res, 405, { error: "Method not allowed" })
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
				if (path === "/api/vault/sweep") return sendJson(res, 200, vaultSweepDto(await vault.sweepNow()))
				await vault.redeemAll()
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
			chainLabels: Object.fromEntries(op.chains.map((id) => [id, chainLabel(id)])),
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

		// Per provided side: a non-empty curve on an absent side *enables* it
		// (one-sided LP opened by the operator), an empty curve on a present
		// side *disables* it (back to one-sided LP), an empty curve on an
		// absent side is a no-op. Venue-priced pairs expose neither hook, and
		// same-token markets are ask-only by construction.
		const enabling: Array<{ side: "bid" | "ask"; points: PriceCurvePoint[] }> = []
		const disabling: Array<"bid" | "ask"> = []
		for (const side of ["bid", "ask"] as const) {
			const points = side === "bid" ? update.bidPriceCurve : update.askPriceCurve
			const current = side === "bid" ? strategy.bid : strategy.ask
			if (points === undefined) continue
			if (points.length === 0) {
				if (!current) continue
				if (!strategy.disableSide) {
					return sendJson(res, 409, {
						error: strategy.sameToken
							? "Same-token markets are ask-only — deleting the ask would remove the market; remove the pair from the config instead"
							: strategy.referenceOnly
								? "The curve is the reference price feed — remove the pair from the config to retire it"
								: `The ${side} side of this strategy is not editable (venue-priced)`,
					})
				}
				disabling.push(side)
			} else if (!current) {
				if (!strategy.enableSide) {
					return sendJson(res, 409, {
						error:
							strategy.sameToken && side === "bid"
								? "Same-token markets are ask-only — the bid side cannot be enabled"
								: `The ${side} side of this strategy is not editable (venue-priced)`,
					})
				}
				enabling.push({ side, points })
			}
		}
		const bidAfter = disabling.includes("bid") ? false : Boolean(strategy.bid) || enabling.some((e) => e.side === "bid")
		const askAfter = disabling.includes("ask") ? false : Boolean(strategy.ask) || enabling.some((e) => e.side === "ask")
		if (!bidAfter && !askAfter) {
			return sendJson(res, 409, {
				error: "A market needs at least one side — remove the pair from the config to retire it",
			})
		}

		// Apply all-or-nothing: validate every curve before touching any policy.
		const sides: Array<{ label: "bid" | "ask"; policy: FillerPricePolicy; points: PriceCurvePoint[] }> = []
		if (update.bidPriceCurve?.length && strategy.bid)
			sides.push({ label: "bid", policy: strategy.bid, points: update.bidPriceCurve })
		if (update.askPriceCurve?.length && strategy.ask)
			sides.push({ label: "ask", policy: strategy.ask, points: update.askPriceCurve })
		const enabled: Array<{ side: "bid" | "ask"; policy: FillerPricePolicy }> = []
		try {
			for (const side of sides) {
				// Constructing a throwaway policy runs full validation without mutating.
				void new FillerPricePolicy({ points: side.points })
			}
			for (const enable of enabling) {
				enabled.push({ side: enable.side, policy: new FillerPricePolicy({ points: enable.points }) })
			}

			// Live edits keep the same startup invariants. A crossed book is
			// allowed (the sides are quoted independently; the crossed region
			// never fills), but a same-token ask must stay strictly below par.
			const nextAsk = update.askPriceCurve?.length
				? new FillerPricePolicy({ points: update.askPriceCurve })
				: strategy.ask
			if (strategy.sameToken && nextAsk) {
				for (const point of nextAsk.getPoints()) {
					if (new Decimal(point.price).gte(1)) {
						throw new Error(
							`same-token ask prices must be strictly below 1 — '${point.price}' would fill at or above par`,
						)
					}
				}
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
		for (const side of disabling) {
			strategy.disableSide!(side)
			if (side === "bid") strategy.bid = undefined
			else strategy.ask = undefined
			this.logger.warn({ strategy: index, side }, "Trading direction disabled from the UI (one-sided LP)")
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
		if (update.bidPriceCurve !== undefined) {
			if (update.bidPriceCurve.length) pair.bidPriceCurve = update.bidPriceCurve
			else delete pair.bidPriceCurve
		}
		if (update.askPriceCurve !== undefined) {
			if (update.askPriceCurve.length) pair.askPriceCurve = update.askPriceCurve
			else delete pair.askPriceCurve
		}
		return this.persistConfig()
	}

	/**
	 * POST /api/strategies — adds a market. The candidate is validated against
	 * the FULL prospective config (duplicate/reverse orientation, USD anchor
	 * graph, symbol resolution on the running chains) before anything mutates,
	 * hydrated into the running engine when possible, and persisted either way.
	 */
	private async handleMarketAdd(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const op = this.operator!
		let body: {
			token0?: string
			token1?: string
			maxOrderSize?: string
			bidPriceCurve?: PriceCurvePoint[]
			askPriceCurve?: PriceCurvePoint[]
			assets?: Record<string, AssetDefinition>
		}
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (!body.token0?.trim() || !body.token1?.trim()) {
			return sendJson(res, 400, { error: "token0 and token1 are required" })
		}

		const candidate: PairConfig = {
			token0: body.token0.trim(),
			token1: body.token1.trim(),
			...(String(body.maxOrderSize ?? "").trim() ? { maxOrderSize: String(body.maxOrderSize).trim() } : {}),
			...(body.bidPriceCurve?.length ? { bidPriceCurve: body.bidPriceCurve } : {}),
			...(body.askPriceCurve?.length ? { askPriceCurve: body.askPriceCurve } : {}),
		}
		const assets = body.assets && Object.keys(body.assets).length > 0 ? body.assets : undefined
		const mergedAssets = { ...(op.config.assets ?? {}) }
		const next = [...(op.config.pairs ?? []), candidate]
		try {
			if (assets) {
				validateAssetDefinitions(assets)
				// Redefining a known symbol under a running engine would silently
				// repoint its fills to another contract.
				const known = new AssetRegistry(new ChainConfigService({}), op.config.assets)
				for (const symbol of Object.keys(assets)) {
					if (known.hasSymbol(symbol)) {
						throw new Error(`assets: '${symbol}' is already defined and cannot be redefined at runtime`)
					}
				}
				Object.assign(mergedAssets, assets)
			}
			const hasVenuePricing = Boolean(op.config.vault?.uniswapV4?.positions?.length)
			validatePairConfigs(next, mergedAssets, hasVenuePricing)
			const registry = new AssetRegistry(new ChainConfigService({}), mergedAssets)
			assertPairSymbolsResolve(next, registry, op.chains.map((id) => formatChainKey(id)))
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		let strategy: AdminStrategy | null = null
		try {
			if (op.addPair) strategy = await op.addPair(candidate, assets, (op.config.pairs ?? []).length)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		// The capability path owns the config: PairController reassigned
		// `config.pairs` (and assets) itself. Writing `next` over it would be a
		// second source of truth that only agrees by construction — mutate here
		// solely in the restart-needed fallback, where nothing else will.
		if (!op.addPair) {
			op.config.pairs = next
			if (assets) op.config.assets = mergedAssets
		}
		const persisted = this.persistConfig()
		const applied = Boolean(op.addPair)
		this.logger.warn(
			{ pair: `${candidate.token0}/${candidate.token1}`, applied, customAssets: assets ? Object.keys(assets) : undefined },
			"Market added by operator",
		)
		return sendJson(res, 200, {
			applied,
			restartNeeded: !applied,
			persisted,
			strategy: strategy ? serializeStrategy(strategy) : null,
		})
	}

	/**
	 * PUT /api/strategies/:index — edits a live market's per-order cap. The
	 * engine reads `maxOrderSize` while sizing each order, so a new cap binds on
	 * the next evaluation; it is persisted to the pair's config entry either way.
	 */
	private async handleMarketUpdate(req: IncomingMessage, res: ServerResponse, index: number): Promise<void> {
		const op = this.operator!
		const strategy = op.strategies.find((s) => s.index === index)
		if (!strategy) return sendJson(res, 404, { error: `No strategy with index ${index}` })

		let body: Record<string, unknown>
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		const { maxOrderSize, ...rest } = body
		if (Object.keys(rest).length > 0) {
			return sendJson(res, 400, { error: `Unknown fields: ${Object.keys(rest).join(", ")}` })
		}
		if (maxOrderSize === undefined) {
			return sendJson(res, 400, { error: "Provide maxOrderSize" })
		}
		if (strategy.referenceOnly) {
			return sendJson(res, 409, {
				error: "Reference-only markets never fill orders — their order cap is never consulted",
			})
		}
		const value = String(maxOrderSize).trim()
		let parsed: Decimal
		try {
			parsed = new Decimal(value)
		} catch {
			return sendJson(res, 400, { error: `maxOrderSize must be a decimal string, got '${value}'` })
		}
		if (!parsed.isFinite() || parsed.lte(0)) {
			return sendJson(res, 400, { error: `maxOrderSize must be a positive number, got '${value}'` })
		}

		strategy.setMaxOrderSize?.(value)
		strategy.maxOrderSize = value
		const pair = op.config.pairs?.[strategy.pairIndex]
		if (pair) pair.maxOrderSize = value
		const persisted = this.persistConfig()
		const applied = Boolean(strategy.setMaxOrderSize)
		this.logger.warn(
			{ strategy: index, pair: `${strategy.token0}/${strategy.token1}`, maxOrderSize: value, applied },
			"Max order size updated by operator",
		)
		return sendJson(res, 200, {
			...serializeStrategy(strategy),
			applied,
			restartNeeded: !applied,
			persisted,
		})
	}

	/**
	 * DELETE /api/strategies/:index/max-order-size — removes a market's per-order
	 * cap, leaving it uncapped. Its own route rather than a null on the PUT:
	 * DELETE /api/strategies/:index already means "remove the market", and a cap
	 * removal that a typo could turn into a market removal is not a trade worth
	 * making for one fewer endpoint.
	 *
	 * Idempotent — clearing an already-uncapped market succeeds and reports the
	 * same state, so the UI does not have to know which it is.
	 */
	private async handleMaxOrderSizeClear(res: ServerResponse, index: number): Promise<void> {
		const op = this.operator!
		const strategy = op.strategies.find((s) => s.index === index)
		if (!strategy) return sendJson(res, 404, { error: `No strategy with index ${index}` })
		if (strategy.referenceOnly) {
			return sendJson(res, 409, {
				error: "Reference-only markets never fill orders — their order cap is never consulted",
			})
		}

		const previous = strategy.maxOrderSize
		strategy.clearMaxOrderSize?.()
		strategy.maxOrderSize = undefined
		const pair = op.config.pairs?.[strategy.pairIndex]
		if (pair) pair.maxOrderSize = undefined
		const persisted = this.persistConfig()
		const applied = Boolean(strategy.clearMaxOrderSize)
		this.logger.warn(
			{ strategy: index, pair: `${strategy.token0}/${strategy.token1}`, previous, applied },
			"Max order size removed by operator — market is now uncapped",
		)
		return sendJson(res, 200, {
			...serializeStrategy(strategy),
			applied,
			restartNeeded: !applied,
			persisted,
		})
	}

	/**
	 * DELETE /api/strategies/:index — removes a market. The remaining config
	 * must still validate (at least one market, no orphaned USD anchor) before
	 * anything mutates. Funds are never touched: vault treasury is per-asset
	 * and stays configured regardless of markets.
	 */
	private async handleMarketRemove(res: ServerResponse, index: number): Promise<void> {
		const op = this.operator!
		const strategy = op.strategies.find((s) => s.index === index)
		if (!strategy) return sendJson(res, 404, { error: `Unknown strategy ${index}` })
		const pairs = op.config.pairs ?? []
		const { pairIndex } = strategy
		try {
			const remaining = pairs.filter((_, i) => i !== pairIndex)
			if (remaining.length === 0) {
				throw new Error("The last market cannot be removed live — edit the config and restart instead")
			}
			const hasVenuePricing = Boolean(op.config.vault?.uniswapV4?.positions?.length)
			validatePairConfigs(remaining, op.config.assets, hasVenuePricing)
			await op.removePair?.(index)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}
		if (!op.removePair) pairs.splice(pairIndex, 1)
		const persisted = this.persistConfig()
		const applied = Boolean(op.removePair)
		this.logger.warn(
			{ pair: `${strategy.token0}/${strategy.token1}`, applied },
			"Market removed by operator",
		)
		return sendJson(res, 200, { applied, restartNeeded: !applied, persisted })
	}

	/**
	 * Regenerates the config file from the (mutated) running config. Chain rows
	 * carry no chain id, so each is re-labelled from the known mapping — without
	 * it a rewritten file loses track of which `[[chains]]` entry is which.
	 */
	private persistConfig(): boolean {
		const op = this.operator!
		// An embedded filler started from a config object has nowhere to write
		// back to. Edits still apply live; they just do not outlive the process,
		// which every caller already surfaces as `persisted: false`.
		if (!op.configPath) return false
		try {
			const chainComments = this.configChainIds().map((id) => chainLabel(id))
			writeConfigFileAtomic(op.configPath, emitFillerToml(op.config, { chainComments }))
			return true
		} catch (err) {
			this.logger.warn({ err, configPath: op.configPath }, "Change applied in memory but could not be persisted")
			return false
		}
	}

	/** Chain ids positionally aligned with `config.chains`; see `configuredChainIds`. */
	private configChainIds(): number[] {
		const op = this.operator!
		const ids = this.configuredChainIds ?? op.chains
		// Positional: a shorter mapping (config edited outside the UI) leaves the
		// trailing rows unidentified rather than mislabelling every row.
		return op.config.chains.map((_, index) => ids[index] ?? 0)
	}

	/** GET /api/chains — the editable chain set plus the catalog of chains that can be added. */
	private handleChainsGet(res: ServerResponse): void {
		const op = this.operator!
		const ids = this.configChainIds()
		const running = new Set(op.chains)
		const watchOnly = op.config.simplex.watchOnly
		const globalWatchOnly = typeof watchOnly === "boolean"
		// RPC URLs are returned unmasked: the editor round-trips them, and masked
		// values would be written straight back into the config. Same trust
		// boundary as the wizard, which collects private keys over this listener.
		const chains: ChainRowDto[] = op.config.chains.map((chain, index) => {
			const chainId = ids[index]
			return {
				chainId,
				stateMachineId: formatChainKey(chainId),
				label: chainLabel(chainId),
				rpcUrls: chain.rpcUrls,
				bundlerUrl: chain.bundlerUrl,
				watchOnly: globalWatchOnly
					? (watchOnly as boolean)
					: Boolean((watchOnly as Record<string, boolean> | undefined)?.[String(chainId)]),
				running: running.has(chainId),
			}
		})
		// One network per filler (the Hyperbridge endpoint and the deployments
		// differ), so the catalog is scoped the way the wizard scopes it.
		const network: InitNetwork = chains.some(
			(row) => INIT_CHAINS.find((meta) => meta.chainId === row.chainId)?.network === "testnet",
		)
			? "testnet"
			: "mainnet"
		const dto: ChainsDto = { chains, catalog: chainsForNetwork(network), network, globalWatchOnly }
		return sendJson(res, 200, dto)
	}

	/**
	 * PUT /api/chains — replaces the chain set (selection, quorum RPC endpoints,
	 * bundler, watch-only). Chain topology is wired at boot — clients, event
	 * monitors, delegation and balance tracking all key off it — so the new set
	 * is validated and persisted, then takes effect on the next start.
	 *
	 * Every candidate is checked the way boot would check it: reachable RPCs
	 * reporting the claimed chain, confirmation coverage, pair symbols still
	 * resolving somewhere, and no funding venue left stranded on a dropped chain.
	 */
	private async handleChainsUpdate(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const op = this.operator!
		let body: { chains?: Array<Record<string, unknown>> }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (!Array.isArray(body.chains) || body.chains.length === 0) {
			return sendJson(res, 400, { error: "Provide chains as a non-empty array — the filler needs at least one chain" })
		}

		const rows: Array<{ chainId: number; rpcUrls: string[]; bundlerUrl: string; watchOnly: boolean }> = []
		try {
			for (const row of body.chains) {
				const chainId = Number(row.chainId)
				if (!Number.isInteger(chainId) || chainId <= 0) {
					throw new Error(`Invalid chainId: ${String(row.chainId)}`)
				}
				if (rows.some((r) => r.chainId === chainId)) {
					throw new Error(`Chain ${chainId} is listed twice`)
				}
				const rpcUrls = (Array.isArray(row.rpcUrls) ? row.rpcUrls : [])
					.map((url) => String(url).trim())
					.filter(Boolean)
				// Same non-empty/distinct-host rule boot applies to every chain.
				validateRpcUrls(rpcUrls)
				const bundlerUrl = String(row.bundlerUrl ?? "").trim()
				if (!bundlerUrl) {
					throw new Error(`${chainLabel(chainId)} needs a bundler URL to submit fill UserOperations`)
				}
				rows.push({ chainId, rpcUrls, bundlerUrl, watchOnly: row.watchOnly === true })
			}
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		const chainIds = rows.map((r) => r.chainId)
		const previous = new Map(this.configChainIds().map((id, index) => [id, op.config.chains[index]]))
		const removed = [...previous.keys()].filter((id) => id > 0 && !chainIds.includes(id))

		// Testnet chain ids have no built-in confirmation curve; write the same
		// low-value default the wizard does rather than rejecting the addition.
		const confirmationPolicies = { ...(op.config.confirmationPolicies ?? {}) }
		for (const row of rows) {
			if (INIT_CHAINS.find((meta) => meta.chainId === row.chainId)?.network === "testnet") {
				confirmationPolicies[String(row.chainId)] ??= { points: TESTNET_CONFIRMATION_POINTS }
			}
		}

		try {
			assertConfirmationCoverage(confirmationPolicies, chainIds)
			if (op.config.pairs?.length) {
				const registry = new AssetRegistry(new ChainConfigService({}), op.config.assets)
				assertPairSymbolsResolve(op.config.pairs, registry, chainIds.map(formatChainKey))
			}
			// Funding venues hydrate per chain at boot: one left on a dropped
			// chain would fall back to a default RPC or fail the next start.
			for (const chainId of removed) {
				const chainKey = formatChainKey(chainId)
				if (op.config.vault?.vaults?.some((vault) => vault.chain === chainKey)) {
					throw new Error(
						`${chainLabel(chainId)} still holds a vault entry — remove it from the vault treasury before dropping the chain`,
					)
				}
				if (op.config.vault?.uniswapV4?.positions?.some((position) => position.chain === chainKey)) {
					throw new Error(
						`${chainLabel(chainId)} still holds a Uniswap V4 position — remove it from the config before dropping the chain`,
					)
				}
			}
			// Probe only what the operator newly asserts: an unreachable endpoint
			// or one answering for another chain would brick the next boot.
			await this.probeChainRows(rows, previous)
		} catch (err) {
			return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
		}

		op.config.chains = rows.map(({ rpcUrls, bundlerUrl }) => ({ rpcUrls, bundlerUrl }))
		if (Object.keys(confirmationPolicies).length > 0) op.config.confirmationPolicies = confirmationPolicies
		// A global boolean watchOnly is left as-is: expanding it per chain would
		// stop validateConfig treating the config as all-watch-only, which is
		// what lets a signer-less observer boot at all.
		if (typeof op.config.simplex.watchOnly !== "boolean") {
			const perChain = Object.fromEntries(rows.filter((r) => r.watchOnly).map((r) => [String(r.chainId), true]))
			op.config.simplex.watchOnly = Object.keys(perChain).length > 0 ? perChain : undefined
		}
		this.configuredChainIds = chainIds
		const persisted = this.persistConfig()
		this.logger.warn({ chains: chainIds, removed }, "Chain set updated by operator")
		return sendJson(res, 200, { applied: false, restartNeeded: true, persisted, chains: chainIds, removed })
	}

	/** Verifies every RPC URL that is new to its chain actually answers for that chain. */
	private async probeChainRows(
		rows: Array<{ chainId: number; rpcUrls: string[] }>,
		previous: Map<number, { rpcUrls: string[] } | undefined>,
	): Promise<void> {
		const probes: Array<{ chainId: number; url: string }> = []
		for (const row of rows) {
			const known = new Set(previous.get(row.chainId)?.rpcUrls ?? [])
			for (const url of row.rpcUrls) {
				if (!known.has(url)) probes.push({ chainId: row.chainId, url })
			}
		}
		await Promise.all(
			probes.map(async ({ chainId, url }) => {
				let reported: number
				try {
					reported = await withTimeout(this.deps.fetchChainId(url), PROBE_TIMEOUT_MS, "RPC check")
				} catch (err) {
					throw new Error(`RPC ${url} is unreachable: ${err instanceof Error ? err.message : err}`)
				}
				if (reported !== chainId) {
					throw new Error(`RPC ${url} reports chain ${reported}, expected ${chainId} (${chainLabel(chainId)})`)
				}
			}),
		)
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
		this.operator!.setLogLevel(level as LogLevel)
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

	/**
	 * Registry vault catalog for every chain on the running network (mainnet or
	 * testnet), same source as the setup wizard's. Chains the filler is not
	 * running are included so the treasury editor can show what becomes
	 * available once a chain is enabled; the UI keeps those rows unselectable.
	 */
	private knownVaultCatalog(op: OperatorContext): ConfigDto["knownVaults"] {
		const chainRegistry = new ChainConfigService({})
		const catalog: ConfigDto["knownVaults"] = {}
		const network = runningNetwork(op.chains)
		const running = new Set(op.chains.map((chainId) => formatChainKey(chainId)))
		const stateMachineIds = new Set([...running, ...chainsForNetwork(network).map((meta) => meta.stateMachineId)])
		for (const stateMachineId of stateMachineIds) {
			const vaults = chainRegistry.getKnownVaults(stateMachineId)
			if (vaults.length > 0 || running.has(stateMachineId)) catalog[stateMachineId] = vaults
		}
		return catalog
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
				await op.activity.recordWalletTx({
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
		maxOrderSize: strategy.maxOrderSize,
		bid: strategy.bid?.getPoints(),
		ask: strategy.ask?.getPoints(),
	}
}

/** Catalog label for a chain id, falling back to the bare id for unlisted chains. */
function chainLabel(chainId: number): string {
	return INIT_CHAINS.find((meta) => meta.chainId === chainId)?.label ?? `chain ${chainId}`
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
		// An empty array is meaningful: it disables that side (one-sided LP).
		if (!Array.isArray(curve)) {
			return `${name} must be an array of points`
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

/** Wire shape of a sweep pass: base units formatted once here so the dashboard never sees bigints. */
/** One network per filler: testnet if any running chain is a testnet, else mainnet. */
function runningNetwork(chains: number[]): InitNetwork {
	return chains.some((chainId) => chainByChainId(chainId)?.network === "testnet") ? "testnet" : "mainnet"
}

function vaultSweepDto(result: VaultSweepResult): VaultSweepDto {
	return {
		ok: true,
		submitted: result.submitted.map((tx) => ({
			chain: tx.chain,
			txHash: tx.txHash,
			sponsored: tx.sponsored,
			deposits: tx.deposits.map((d) => ({ vault: d.vault, symbol: d.symbol, amount: formatUnits(d.amount, d.decimals) })),
		})),
		skipped: result.skipped.map((skip) => ({
			chain: skip.chain,
			vault: skip.vault,
			symbol: skip.symbol,
			reason: skip.reason,
			...(skip.walletBalance !== undefined ? { walletBalance: formatUnits(skip.walletBalance, skip.decimals) } : {}),
			...(skip.threshold !== undefined ? { threshold: formatUnits(skip.threshold, skip.decimals) } : {}),
		})),
	}
}
