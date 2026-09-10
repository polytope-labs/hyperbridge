import { lstatSync, unlinkSync, type Stats } from "node:fs"
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { connect } from "node:net"
import { tmpdir } from "node:os"
import { resolve as resolvePath } from "node:path"
import type { Duplex } from "node:stream"
import { Decimal } from "decimal.js"
import { FillerPricePolicy, formatChainKey, parseChainKey, type PriceCurvePoint } from "@/config/interpolated-curve"
import { AssetRegistry, registrySymbols, validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
import { assertPairSymbolsResolve, validatePairConfigs, type PairConfig } from "@/config/pairs"
import { VaultFundingPlanner, type VaultSweepResult } from "@/funding/vault/VaultFundingPlanner"
import { chainByChainId, chainsForNetwork, INIT_CHAINS, nativeTokenSymbol, type InitNetwork } from "@/cli/init/chains"
import { TESTNET_CONFIRMATION_POINTS } from "@/cli/init/state"
import { ChainConfigService } from "@hyperbridge/sdk"
import { assertConfirmationCoverage, type FillerConfigFile, type FillerTomlConfig, type VaultToml } from "@/config/filler-toml"
import { emitFillerToml, writeConfigFileAtomic } from "@/cli/init/emit-toml"
import { formatUnits, isAddress } from "viem"
import { validateRpcUrls, type AllowlistConfig } from "@/services/FillerConfigService"
import { withTimeout, PROBE_TIMEOUT_MS } from "@/cli/init/prompt-utils"
import type { ActivityRecorder } from "@/data/recorder"
import type { ActivityEvent, BidStore, OrderLeg } from "@/data/types"
import type { BalanceProvider } from "../BalanceProvider"
import { getLogger, type LogLevel } from "../Logger"
import { DEFAULT_TUNNEL_RELAY, parseRelayAddress, relayKey, type TunnelControls } from "../tunnel/TunnelService"
import {
	readBody,
	sendJson,
	isLoopbackHost,
	isContainerized,
	hostHeaderAllowed,
	isTunnelled,
	markProvenance,
	provenanceOf,
	type Provenance,
} from "./http-util"
import { matchesLogQuery, type LogQuery, type LogTail } from "./LogStore"
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
	LOG_LEVEL_RANK,
	type AdminStrategyDto,
	type ChainRowDto,
	type ChainsDto,
	type ConfigDto,
	type LogRecordDto,
	type LogRecordLevel,
	type LogsDto,
	type SendTokenOption,
	type StatusInit,
	type StatusOperator,
	type WalletTxDto,
	type VaultSweepDto,
	type OrderHistoryDto,
	type LedgerLeg,
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

/**
 * Where the UI server listens.
 *
 * A TCP port is the CLI's mode and reaches every local user, so it carries the
 * host-header defense and the loopback rules below. A `socketPath` binds a Unix
 * domain socket instead (a named pipe on Windows) — the filesystem decides who
 * may connect, and no web page can open one at all, which is what lets an
 * embedding application (the desktop app) expose the API without a port.
 *
 * The two are alternatives, not layers: a server listens on one or the other.
 */
export type ListenTarget = { host?: string; port: number } | { socketPath: string }

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
	/**
	 * This launch's log history, as read by the Logs page. Absent when nothing
	 * registered a {@link LogStore} — an embedded filler, or a test — and the log
	 * routes then report the page as unavailable rather than showing an empty feed.
	 */
	logs?: LogTail
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
	/** Remote-access tunnel controls; absent when the binary runs without a data dir for keys (embedded fillers). */
	tunnel?: TunnelControls
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

const LOGS_UNAVAILABLE = "Log capture is not enabled for this filler"

/** Above this, a log tail is talking to a reader that has stopped reading; frames are dropped instead of queued. */
const MAX_LOG_STREAM_BACKLOG_BYTES = 1_000_000

/** The most records one `GET /api/logs` will return, whatever the caller asks for. */
const MAX_LOG_PAGE = 2000

/** `?level=&q=&after=&limit=` for both log routes. Unparseable values fall back rather than 400. */
function logQueryFrom(url: string | undefined): LogQuery {
	const params = new URL(url ?? "/", "http://localhost").searchParams
	const level = params.get("level")
	const q = params.get("q")?.trim()
	const after = Number(params.get("after"))
	const limit = Number(params.get("limit"))
	return {
		level: level && level in LOG_LEVEL_RANK ? (level as LogRecordLevel) : undefined,
		q: q ? q.slice(0, 200) : undefined,
		after: Number.isFinite(after) && after > 0 ? after : undefined,
		limit: Number.isFinite(limit) && limit > 0 ? Math.min(limit, MAX_LOG_PAGE) : MAX_LOG_PAGE,
	}
}

const UI_NOT_BUILT_HTML = `<!doctype html><meta charset="utf-8"><title>simplex</title>
<body style="font-family:system-ui;margin:4rem auto;max-width:32rem">
<h1>UI not built</h1><p>The simplex web UI is missing from this build.
Run <code>pnpm ui:build</code> (or a full <code>pnpm build</code>) and restart.</p>
<p>The JSON API under <code>/api</code> is unaffected.</p></body>`

/**
 * `sun_path` in `sockaddr_un` is a fixed-size field: 108 bytes on Linux, 104 on
 * macOS and the BSDs, terminating NUL included. Past it bind(2) fails with a
 * message naming neither the limit nor the offending path.
 *
 * It bites in practice rather than in theory: a desktop application's natural
 * home for such a file is its user-data directory, which on macOS is already
 * `~/Library/Application Support/<app>/` before a filename is added.
 */
const SUN_PATH_MAX_BYTES = process.platform === "darwin" ? 103 : 107

/**
 * Windows names a pipe `\\.\pipe\name`, which is not a filesystem path.
 *
 * Gated on the platform as well as the shape: on Linux such a string is just an
 * oddly-named file, and treating it as a pipe there would skip the path
 * resolution and the cleanup that every other path gets.
 */
function isWindowsPipe(path: string): boolean {
	return process.platform === "win32" && /^\\\\[.?]\\pipe\\/i.test(path)
}

/** Names what is sitting at a socket path, for an error the operator can act on. */
function describeEntry(entry: Stats): string {
	if (entry.isSymbolicLink()) return "a symbolic link"
	if (entry.isDirectory()) return "a directory"
	if (entry.isFIFO()) return "a FIFO"
	if (entry.isFile()) return "a regular file"
	return "not a socket"
}

/**
 * Refuses an over-long socket path up front, with the limit and a way out,
 * rather than letting bind(2) produce an opaque failure.
 *
 * Refusing beats silently relocating to a shorter path: the caller uses this
 * path to find the daemon again, so moving it would trade a clear error at
 * startup for a daemon nothing can attach to.
 */
function assertSocketPathFits(path: string): void {
	// A Windows pipe is not a sockaddr_un; its own cap is the 256-character pipe
	// name, which no plausible path approaches.
	if (process.platform === "win32") return
	// Bytes, not characters: a non-ASCII path spends more of the field than it looks.
	const bytes = Buffer.byteLength(path)
	if (bytes <= SUN_PATH_MAX_BYTES) return
	throw new Error(
		`UI socket path is ${bytes} bytes, over this platform's ${SUN_PATH_MAX_BYTES}-byte limit for a Unix socket: ${path}. ` +
			`Use a shorter path in a directory only this user can write — on Linux $XDG_RUNTIME_DIR ` +
				`(${process.env.XDG_RUNTIME_DIR ?? "/run/user/<uid>"}) is both short and already private. ` +
				`Avoid a shared directory such as ${tmpdir()}: anything that can create names there can take this address first.`,
	)
}

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
	/** Open log tails, each mapped to the unsubscribe that detaches it from the buffer. */
	private logClients = new Map<ServerResponse, () => void>()
	private activityListener?: (event: ActivityEvent) => void
	private boundLoopback = true
	/** How connections this server accepted arrived; stamped onto each socket. */
	private listenProvenance: Provenance = "tcp"
	/** Set while a Unix socket is bound, so shutdown can take the path back down with it. */
	private socketPath?: string
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
		// Prepended so the stamp lands before http's own connection listener attaches
		// a parser, which makes `provenanceOf(req.socket)` total by the time `handle`
		// runs. `accept()` stamps its channels first and `markProvenance` keeps the
		// first stamp, so an injected connection is never relabelled as a listened-for
		// one. Tagging the socket, rather than reading what the server happens to be
		// listening on, is what keeps the rules right for a server serving both.
		this.server.prependListener("connection", (socket) => markProvenance(socket, this.listenProvenance))
	}

	/**
	 * Serves one connection handed over by the remote-access tunnel. It never
	 * touches the network: the device's SSH channel becomes this server's socket
	 * directly, which is also what lets `handle` tell the two apart.
	 */
	accept(socket: Duplex): boolean {
		if (!this.server.listening) return false
		// The tunnel is the only caller, and this is the one place that holds
		// regardless of how the channel was built — so the tunnel provenance is
		// settled here, ahead of the listener stamp the emit below would otherwise
		// apply.
		markProvenance(socket, "tunnel")
		this.server.emit("connection", socket)
		return true
	}

	/** Resolves with the bound port once listening (pass port 0 for an ephemeral port). */
	start(port: number, host?: string): Promise<number>
	/** Resolves once listening; the bound port for a TCP target, 0 for a Unix socket. */
	start(target: ListenTarget): Promise<number>
	async start(target: number | ListenTarget, host = "127.0.0.1"): Promise<number> {
		if (typeof target === "number") return this.listenOnPort(target, host)
		if ("socketPath" in target) return this.listenOnSocket(target.socketPath)
		return this.listenOnPort(target.port, target.host ?? host)
	}

	/**
	 * Binds a Unix domain socket (a named pipe on Windows) rather than a port.
	 * Node's `listen(path)` speaks HTTP over one natively, so the route table,
	 * the handlers and the SSE stream are the same code either way.
	 *
	 * Resolves 0: there is no port to report. A listening TCP server never reports
	 * 0 either, so the two are not confusable.
	 */
	private async listenOnSocket(socketPath: string): Promise<number> {
		// A named pipe name is not a filesystem path; resolving one would mangle it.
		const path = isWindowsPipe(socketPath) ? socketPath : resolvePath(socketPath)
		assertSocketPathFits(path)
		// Refused rather than attempted: `listen` throws ERR_SERVER_ALREADY_LISTEN here,
		// and the provenance set below decides whether the DNS-rebinding check runs.
		// Setting it for a bind that cannot happen would relabel a live TCP listener's
		// connections as socket-arrived and switch that check off for them.
		if (this.server.listening) {
			throw new Error("This UI server is already listening; build a second UiServer for a second address")
		}
		await this.clearStaleSocket(path)
		await new Promise<void>((resolve, reject) => {
			this.server.once("error", reject)
			this.listenPrivate(path, () => {
				// Assigned here, not before the bind: these describe how requests on this
				// server are judged, so they must describe a listener that came up. The
				// callback runs before the loop can deliver a connection, so nothing is
				// ever served under the previous values.
				//
				// A socket file is reachable by strictly fewer callers than loopback is, so
				// the loopback branch of the host rule is the right default for anything
				// that consults it — though `handle` skips that check outright here.
				this.boundLoopback = true
				this.listenProvenance = "unix"
				resolve()
			})
		})
		this.socketPath = path
		try {
			this.assertSocketIsPrivate(path)
		} catch (err) {
			// Never keep serving a fund-moving API on a socket we cannot prove is
			// owner-only. The bind succeeded, so it has to be taken back down.
			this.server.close()
			this.unlinkSocket()
			throw err
		}
		this.logger.info({ bind: path }, `Simplex UI available on the socket at ${path}`)
		return 0
	}

	/**
	 * Binds the socket with a umask that makes it `0600` at creation.
	 *
	 * It cannot be narrowed after the fact instead. libuv creates the file with
	 * `0777 & ~umask` — 0775 under the common `umask 002` — and Linux checks that
	 * mode at connect(2) and never again, so a local user who connects in the gap
	 * before a follow-up chmod keeps a fully privileged, unauthenticated session for
	 * the life of the daemon: tightening the mode does not revoke a connection that
	 * already exists. The gap is around a millisecond, which a connect loop wins
	 * reliably. The mode has to be right before the socket is reachable at all.
	 *
	 * `process.umask` is process-wide, which is why this wraps only the synchronous
	 * `listen` call: libuv binds inside it, and no other JavaScript in this process
	 * can run in between. It throws on a worker thread, hence the guard.
	 */
	private listenPrivate(path: string, onListening: () => void): void {
		if (process.platform === "win32" || isWindowsPipe(path)) {
			// A named pipe carries no file mode; see docs/ai/Decisions.md.
			this.server.listen(path, onListening)
			return
		}
		let previous: number | undefined
		try {
			previous = process.umask(0o177)
		} catch {
			previous = undefined
		}
		try {
			this.server.listen(path, onListening)
		} finally {
			if (previous !== undefined) process.umask(previous)
		}
	}

	/**
	 * Proves the socket is owner-only, and refuses to serve when it is not.
	 *
	 * `listenPrivate` has already made it `0600`; this is purely the post-condition.
	 * It asserts rather than repairs on purpose: a chmod here would "fix" the mode
	 * only after the socket had been reachable at the wrong one, which is precisely
	 * the window `listenPrivate` exists to close — so a repair would hide the very
	 * regression this checks for, while leaving the vulnerability in place.
	 *
	 * `lstat`, not `stat`, so a symlink swapped in at the path is rejected rather
	 * than followed.
	 *
	 * Fails closed deliberately. This mode is the entire access control for the
	 * socket listen mode, and an earlier version logged a warning and kept serving.
	 */
	private assertSocketIsPrivate(path: string): void {
		if (process.platform === "win32" || isWindowsPipe(path)) return
		const entry = lstatSync(path)
		if (!entry.isSocket()) {
			throw new Error(`${path} is ${describeEntry(entry)}, not the socket just bound — refusing to serve`)
		}
		const mode = entry.mode & 0o777
		if (mode !== 0o600) {
			throw new Error(
				`${path} was created mode 0${mode.toString(8)} rather than 0600, so other users could reach the UI. ` +
					"A default ACL on the containing directory is the usual cause; use a directory without one.",
			)
		}
	}

	/**
	 * Clears a socket file left behind by a run that was killed before it could
	 * remove its own; without this a `SIGKILL`ed predecessor makes every later
	 * start fail with EADDRINUSE.
	 *
	 * Existence proves nothing — a live socket has a file too — so the test is to
	 * dial it. `ECONNREFUSED` means the file outlived its listener and is a corpse.
	 * Anything that answers belongs to a running instance, and taking its path
	 * would silently steal its clients, so that is an error instead. That doubles
	 * as the single-instance lock an embedding application wants.
	 *
	 * Windows needs none of it: a named pipe is refcounted by its handles and
	 * vanishes with the process that made it, so nothing stale can exist and the
	 * existence check below returns first.
	 */
	private async clearStaleSocket(path: string): Promise<void> {
		// `lstat`, not `existsSync`: existsSync follows symlinks, so a dangling one
		// reads as absent, nothing is cleaned up, and the bind then fails EADDRINUSE.
		const entry = lstatSync(path, { throwIfNoEntry: false })
		if (!entry) return
		// The type test comes before the probe, not after: connect(2) answers
		// ECONNREFUSED for a regular file, a FIFO and a directory exactly as it does
		// for an orphaned socket, so probing alone would read an operator's file as a
		// corpse and delete it. Only a socket is ever a candidate for removal.
		if (!entry.isSocket()) {
			throw new Error(`${path} already exists and is ${describeEntry(entry)}; refusing to remove it`)
		}
		const code = await new Promise<string | undefined>((resolve) => {
			const probe = connect(path)
			const settle = (result: string | undefined) => {
				probe.destroy()
				resolve(result)
			}
			probe.once("connect", () => settle(undefined))
			probe.once("error", (err) => settle((err as NodeJS.ErrnoException).code ?? "UNKNOWN"))
		})
		// Vanished between the check and the dial: nothing to clear.
		if (code === "ENOENT") return
		if (code === undefined) {
			throw new Error(`Another simplex is already serving its UI on ${path}`)
		}
		if (code !== "ECONNREFUSED") {
			// EACCES on somebody else's socket, say. Not ours to delete.
			throw new Error(`Cannot tell whether ${path} is in use (${code}); remove it by hand if no simplex is running`)
		}
		try {
			unlinkSync(path)
			this.logger.warn({ path }, "Removed a stale UI socket left behind by a previous run")
		} catch (err) {
			if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err
		}
	}

	private listenOnPort(port: number, host: string): Promise<number> {
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
		return new Promise((resolve, reject) => {
			this.server.once("error", reject)
			this.server.listen(port, host, () => {
				// Set once the listener is actually up: these say how this server judges
				// requests, so they must never describe a bind that did not happen.
				this.boundLoopback = isLoopbackHost(host)
				this.listenProvenance = "tcp"
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
		for (const [client, unsubscribe] of this.logClients) {
			unsubscribe()
			client.end()
		}
		this.logClients.clear()
		this.server.close()
		this.unlinkSocket()
	}

	/**
	 * Removes the socket file on the way out, so the next run has nothing to
	 * recover. `server.close()` unlinks too, but only once it has drained every
	 * connection; doing it here frees the path the moment we stop serving and
	 * covers a close that never completes.
	 */
	private unlinkSocket(): void {
		const path = this.socketPath
		if (!path) return
		this.socketPath = undefined
		if (isWindowsPipe(path)) return
		try {
			unlinkSync(path)
		} catch (err) {
			if ((err as NodeJS.ErrnoException).code !== "ENOENT") {
				this.logger.debug({ err, path }, "Could not remove the UI socket")
			}
		}
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

		// Framing defense: the API is unauthenticated by design — the bind is the
		// boundary — so a page that frames this UI never needs to read or script
		// it. It only needs the operator to tap through an invisible overlay: the
		// click lands in the real UI, same-origin, with its own X-Simplex-UI header.
		// That reaches pause, reset-halt and vault sweep/redeem, which are one
		// click each. `frame-ancestors` is the directive browsers honour today;
		// X-Frame-Options is the fallback for older WebViews that ignore CSP.
		// Set here, before anything can return, so 403s carry it too — and via
		// setHeader so every writeHead downstream merges rather than drops it.
		res.setHeader("Content-Security-Policy", "frame-ancestors 'none'")
		res.setHeader("X-Frame-Options", "DENY")

		const provenance = provenanceOf(req.socket)

		// DNS-rebinding defense: an attacker page resolving its own domain to
		// this address becomes same-origin and could drive every endpoint,
		// including /api/send. A rebound origin always carries its DNS name in
		// Host, so only IP-literal/localhost Hosts are served.
		//
		// Skipped, not relaxed, for a connection that arrived on a Unix socket:
		// there is no name to rebind onto one and no browser that can open one, so
		// the check defends nothing there — while an HTTP client over a socket puts
		// whatever it likes in Host (node:http sends "localhost", others send the
		// URL's authority), which would make an arbitrary base URL a 403.
		if (provenance !== "unix" && !hostHeaderAllowed(req.headers.host, this.boundLoopback)) {
			return sendJson(res, 403, { error: "Host header is not allowed" })
		}

		// CSRF hygiene: a cross-origin page can't set this header without a
		// preflight, and no CORS headers are ever emitted.
		if (method !== "GET" && method !== "HEAD" && req.headers["x-simplex-ui"] !== "1") {
			return sendJson(res, 403, { error: "Missing X-Simplex-UI header" })
		}

		// A device on the tunnel reaches this server with exactly the operator's
		// privileges, so remote access cannot be managed from there: pairing a
		// second key would otherwise survive revoking the first, and repointing
		// the relay would move the tunnel to one the holder runs.
		if (path.startsWith("/api/tunnel") && method !== "GET" && method !== "HEAD" && isTunnelled(req.socket)) {
			return sendJson(res, 403, {
				error: "Remote access can only be changed from the machine running Simplex",
			})
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
			const chainRegistry = new ChainConfigService({})
			const vaultLabel = (chainId: number | null, address: string | null): string | null => {
				if (chainId === null || !address) return null
				const known = chainRegistry.getKnownVaults(formatChainKey(chainId))
				return known.find((vault) => vault.address.toLowerCase() === address.toLowerCase())?.label ?? null
			}
			// Share tokens are named after their underlying (stataUSDC, ycNGN): the logo
			// comes from the underlying's symbol, and the vault badge says it is a share.
			const legOf = (symbol: string | null | undefined, amount: string | null | undefined, vault: boolean): LedgerLeg | null =>
				symbol && amount ? { symbol, amount, decimals: null, icon: vault ? underlyingOf(symbol) : symbol, vault } : null
			const orderLeg = (leg: OrderLeg | undefined): LedgerLeg | null =>
				leg ? { symbol: leg.symbol ?? leg.token, amount: leg.amount, decimals: leg.decimals, icon: leg.symbol ?? "", vault: false } : null
			const txs: WalletTxDto[] = [
				...walletTxs.map(({ tokenIn, amountIn, ...tx }) => {
					const vaultTx = tx.kind === "sweep" || tx.kind === "redeem"
					return {
						...tx,
						id: `wallet-${tx.id}`,
						label: vaultTx ? vaultLabel(tx.chainId, tx.to) : null,
						in: legOf(tokenIn, amountIn, tx.kind === "sweep"),
						out: legOf(tx.token, tx.amount, tx.kind === "redeem"),
					}
				}),
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
					label: null,
					in: orderLeg(event.order?.inputs[0]),
					out: orderLeg(event.order?.outputs[0]),
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
				tunnel: op.tunnel ? { enabled: op.tunnel.status().enabled, devices: op.tunnel.status().devices.length } : undefined,
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

		if (path === "/api/logs") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			const logs = this.operator!.logs
			if (!logs) return sendJson(res, 409, { error: LOGS_UNAVAILABLE })
			const stats = logs.stats()
			const dto: LogsDto = {
				level: this.operator!.config.simplex.logging ?? "info",
				capacity: stats.capacity,
				captured: stats.captured,
				persisted: stats.persisted,
				path: stats.path,
				records: await logs.recent(logQueryFrom(req.url)),
			}
			return sendJson(res, 200, dto)
		}

		if (path === "/api/logs/stream") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
			if (!this.operator!.logs) return sendJson(res, 409, { error: LOGS_UNAVAILABLE })
			return await this.streamLogs(req, res)
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

		if (path === "/api/tunnel") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			const tunnel = this.operator!.tunnel
			if (!tunnel) return sendJson(res, 404, { error: "Remote access is not available in this filler" })
			if (method === "GET") return sendJson(res, 200, { ...tunnel.status(), readOnly: isTunnelled(req.socket) })
			if (method === "PUT") return this.handleTunnelUpdate(req, res, tunnel)
			return sendJson(res, 405, { error: "Method not allowed" })
		}

		if (path === "/api/tunnel/devices" || path === "/api/tunnel/devices/revoke") {
			if (this.mode !== "operator") return sendJson(res, 409, { error: "Filler is not running" })
			if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })
			const tunnel = this.operator!.tunnel
			if (!tunnel) return sendJson(res, 404, { error: "Remote access is not available in this filler" })
			let body: { label?: unknown; fingerprint?: unknown; publicKey?: unknown }
			try {
				body = JSON.parse(await readBody(req))
			} catch {
				return sendJson(res, 400, { error: "Invalid JSON body" })
			}
			if (path === "/api/tunnel/devices") {
				if (typeof body.label !== "string" || !body.label.trim()) return sendJson(res, 400, { error: "label is required" })
				if (body.publicKey !== undefined && typeof body.publicKey !== "string") {
					return sendJson(res, 400, { error: "publicKey must be a string" })
				}
				try {
					return sendJson(res, 201, tunnel.addDevice(body.label, body.publicKey))
				} catch (err) {
					return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
				}
			}
			if (typeof body.fingerprint !== "string" || !body.fingerprint) return sendJson(res, 400, { error: "fingerprint is required" })
			const removed = tunnel.removeDevice(body.fingerprint)
			return sendJson(res, removed ? 200 : 404, removed ? { removed: true } : { error: "No device with that fingerprint" })
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

	/**
	 * Enables/disables the tunnel or points it at another relay. The config
	 * block is rewritten so the choice survives a restart; the tunnel applies
	 * it live either way.
	 */
	private async handleTunnelUpdate(req: IncomingMessage, res: ServerResponse, tunnel: TunnelControls): Promise<void> {
		let body: { enabled?: unknown; relay?: unknown }
		try {
			body = JSON.parse(await readBody(req))
		} catch {
			return sendJson(res, 400, { error: "Invalid JSON body" })
		}
		if (body.enabled !== undefined && typeof body.enabled !== "boolean") {
			return sendJson(res, 400, { error: "enabled must be a boolean" })
		}
		if (body.relay !== undefined && typeof body.relay !== "string") {
			return sendJson(res, 400, { error: "relay must be a string" })
		}
		if (typeof body.relay === "string") {
			try {
				parseRelayAddress(body.relay)
			} catch (err) {
				return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
			}
		}
		const update = {
			enabled: body.enabled as boolean | undefined,
			relay: typeof body.relay === "string" ? body.relay.trim() : undefined,
		}
		const op = this.operator!
		const block = { ...(op.config.simplex.tunnel ?? {}) }
		if (update.enabled !== undefined) block.enabled = update.enabled
		if (update.relay !== undefined) {
			// The pin belongs to the relay it was set for; keeping it across a relay
			// change locks remote access out entirely.
			if (block.relayHostKey && relayKey(update.relay) !== relayKey(block.relay ?? DEFAULT_TUNNEL_RELAY)) {
				block.relayHostKey = undefined
			}
			block.relay = update.relay
		}
		op.config.simplex.tunnel = block
		const persisted = this.persistConfig()
		try {
			await tunnel.configure(update)
		} catch (err) {
			return sendJson(res, 500, { error: err instanceof Error ? err.message : String(err) })
		}
		this.logger.warn({ ...update }, "Remote access settings changed from the UI")
		return sendJson(res, 200, { ...tunnel.status(), persisted })
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
	 * Server-sent tail of the log buffer. The client passes the seq of the last
	 * record it holds as `after`, so whatever was logged between its `GET
	 * /api/logs` and this connection is replayed before the live feed starts —
	 * a plain "live only" stream drops exactly the records an operator was
	 * watching for.
	 */
	private async streamLogs(req: IncomingMessage, res: ServerResponse): Promise<void> {
		const logs = this.operator!.logs!
		const query = logQueryFrom(req.url)
		res.writeHead(200, {
			"Content-Type": "text/event-stream",
			"Cache-Control": "no-store",
			Connection: "keep-alive",
		})
		res.write(":ok\n\n")

		// Subscribed before the replay is awaited: reading history can hit the
		// launch file, and anything logged while that I/O is in flight has to be
		// held rather than missed. `seq` orders the two sources on the way out.
		const pending: LogRecordDto[] = []
		let replaying = true
		let lastSent = 0
		let dropped = false

		const send = (record: LogRecordDto) => {
			// A stalled reader (a phone that walked out of signal, mid-tunnel)
			// would otherwise buffer the whole firehose in this process. Dropping
			// frames costs that client some lines; not dropping them costs the
			// filler its memory.
			if (res.writableLength > MAX_LOG_STREAM_BACKLOG_BYTES) {
				dropped = true
				return
			}
			if (dropped) {
				// Tell the client it has a hole rather than letting it splice two
				// non-adjacent stretches together and believe the feed is whole. It
				// re-reads the backfill, which is still on disk.
				dropped = false
				res.write("event: gap\ndata: {}\n\n")
			}
			res.write(`data: ${JSON.stringify(record)}\n\n`)
		}

		const emit = (record: LogRecordDto) => {
			if (record.seq <= lastSent) return
			lastSent = record.seq
			send(record)
		}

		const live: LogQuery = { level: query.level, q: query.q }
		const unsubscribe = logs.subscribe((record) => {
			if (!matchesLogQuery(record, live)) return
			if (replaying) pending.push(record)
			else emit(record)
		})
		this.logClients.set(res, unsubscribe)
		req.on("close", () => {
			unsubscribe()
			this.logClients.delete(res)
		})

		try {
			for (const record of await logs.recent(query)) emit(record)
		} catch {
			// History unreadable: the live tail below still works.
		}
		replaying = false
		for (const record of pending) emit(record)
		pending.length = 0
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
			const tokens: SendTokenOption[] = [{ symbol: nativeTokenSymbol(chainId), address: "native" }]
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

/** The token a vault share token wraps, by naming convention: stataUSDC → USDC, ycNGN → cNGN, aUSDT → USDT. */
function underlyingOf(shareSymbol: string): string {
	for (const known of ["USDC", "USDT", "CNGN", "DAI", "EURC", "ZARP"]) {
		if (shareSymbol.toUpperCase().includes(known)) return known
	}
	return shareSymbol
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
