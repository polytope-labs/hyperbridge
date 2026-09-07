import ssh2, { type ClientChannel, type Client as SshClientType } from "ssh2"
import { getLogger } from "../Logger"
import type { TunnelDeviceDto, TunnelNewDeviceDto, TunnelStatusDto, TunnelState } from "../server/dto"
import { EmbeddedSshServer } from "./EmbeddedSshServer"
import { fingerprintOf, TunnelKeyStore, type StoredKey } from "./keys"

// ssh2 is CommonJS: named imports resolve under vitest's transform but not in
// the ESM binary, where Node cannot see them statically.
const { Client: SshClient } = ssh2

/** The hosted relay every simplex uses unless `[simplex.tunnel] relay` says otherwise. */
export const DEFAULT_TUNNEL_RELAY = "simplex.tunnel.polytope.technology:443"

/** Port the phone forwards to locally; matches the CLI's default UI port so the docs read the same everywhere. */
const LOCAL_FORWARD_PORT = 8686

/** `[simplex.tunnel]` in the config file. */
export interface TunnelConfig {
	/** Off by default: enabling it makes the embedded SSH server reachable from the internet. */
	enabled?: boolean
	/** `host[:port]`; port defaults to 443. */
	relay?: string
	/** `SHA256:…` fingerprint of the relay host key. Without it the key seen on first contact is pinned. */
	relayHostKey?: string
}

/** What the UI server needs from a tunnel, whatever runs behind it. */
export interface TunnelControls {
	status(): TunnelStatusDto
	/** Applies a new enabled flag and/or relay address; reconnects as needed. */
	configure(update: { enabled?: boolean; relay?: string }): Promise<void>
	/** Pairs a device: with `publicKey` the phone keeps its own private key; without, one is minted and returned once. */
	addDevice(label: string, publicKey?: string): TunnelNewDeviceDto
	removeDevice(fingerprint: string): boolean
}

export interface TunnelServiceOptions {
	/** The CLI data directory; keys live under `<dataDir>/tunnel/`. */
	dataDir: string
	config?: TunnelConfig
	/** Where the UI server is bound; the only place device sessions may reach. */
	uiTarget: () => { host: string; port: number }
	/** Test hook: bounds the reconnect delay. */
	maxBackoffMs?: number
}

/** Parses `host[:port]`, IPv6 in brackets, default port 443. */
export function parseRelayAddress(text: string): { host: string; port: number } {
	const trimmed = text.trim()
	if (!trimmed) throw new Error("Relay address is required")
	const bracketed = /^\[([^\]]+)\](?::(\d{1,5}))?$/.exec(trimmed)
	let host: string
	let portText: string | undefined
	if (bracketed) {
		;[, host, portText] = bracketed
	} else {
		const colon = trimmed.lastIndexOf(":")
		// A bare IPv6 literal has several colons and no port; anything with one colon is host:port.
		if (colon > 0 && trimmed.indexOf(":") === colon) {
			host = trimmed.slice(0, colon)
			portText = trimmed.slice(colon + 1)
		} else {
			host = trimmed
		}
	}
	if (!host || /[\s/@]/.test(host)) throw new Error(`Relay address '${text}' is not host[:port]`)
	const port = portText === undefined ? 443 : Number(portText)
	if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error(`Relay port '${portText}' is out of range`)
	return { host, port }
}

/**
 * Keeps one outbound SSH session to the relay and asks it for a remote
 * forward. Every connection the relay hands back is a phone: it goes straight
 * into the embedded SSH server, never onto a local port. Nothing here can
 * affect filling: a tunnel failure is logged, retried with backoff, and
 * otherwise ignored.
 */
export class TunnelService implements TunnelControls {
	private readonly logger = getLogger("tunnel")
	private readonly keys: TunnelKeyStore
	private readonly operatorKey: StoredKey
	private readonly hostKey: StoredKey
	private readonly server: EmbeddedSshServer
	private readonly maxBackoffMs: number
	private config: TunnelConfig
	private client?: SshClientType
	private state: TunnelState = "disabled"
	private port?: number
	private relayFingerprint?: string
	private connectedAt?: number
	private lastError?: string
	private attempt = 0
	private reconnectTimer?: NodeJS.Timeout
	private stopped = false
	/** Set by `verifyRelay` so the generic "verification failed" error does not replace the specific message. */
	private pinMismatch = false

	constructor(private readonly opts: TunnelServiceOptions) {
		this.config = { ...opts.config }
		this.maxBackoffMs = opts.maxBackoffMs ?? 60_000
		this.keys = new TunnelKeyStore(opts.dataDir)
		this.operatorKey = this.keys.operatorKey()
		this.hostKey = this.keys.hostKey()
		this.server = new EmbeddedSshServer({
			hostKey: this.hostKey.privateKey,
			isAuthorized: (fingerprint) => this.keys.isAuthorized(fingerprint),
			target: opts.uiTarget,
		})
	}

	get relay(): string {
		return this.config.relay?.trim() || DEFAULT_TUNNEL_RELAY
	}

	get enabled(): boolean {
		return this.config.enabled === true
	}

	/** Connects when enabled; a no-op otherwise. Safe to call once at boot. */
	start(): void {
		this.stopped = false
		if (!this.enabled) {
			this.state = "disabled"
			return
		}
		this.connect()
	}

	async stop(): Promise<void> {
		this.stopped = true
		this.clearReconnect()
		this.disconnect()
		this.server.close()
		this.state = this.enabled ? "disconnected" : "disabled"
	}

	status(): TunnelStatusDto {
		return {
			enabled: this.enabled,
			state: this.state,
			relay: this.relay,
			relayFingerprint: this.relayFingerprint ?? this.config.relayHostKey ?? this.keys.knownRelay()?.fingerprint,
			port: this.port,
			connectedAt: this.connectedAt,
			lastError: this.lastError,
			hostFingerprint: this.hostKey.fingerprint,
			operatorFingerprint: this.operatorKey.fingerprint,
			devices: this.keys.devices().map(toDeviceDto),
			activeConnections: this.server.connections,
		}
	}

	async configure(update: { enabled?: boolean; relay?: string }): Promise<void> {
		if (update.relay !== undefined) parseRelayAddress(update.relay)
		const relayChanged = update.relay !== undefined && update.relay.trim() !== this.relay
		const next: TunnelConfig = { ...this.config }
		if (update.enabled !== undefined) next.enabled = update.enabled
		if (update.relay !== undefined) next.relay = update.relay.trim()
		this.config = next
		if (!this.enabled) {
			this.clearReconnect()
			this.disconnect()
			this.state = "disabled"
			this.lastError = undefined
			return
		}
		if (relayChanged || !this.client) {
			this.clearReconnect()
			this.disconnect()
			this.attempt = 0
			this.lastError = undefined
			this.connect()
		}
	}

	addDevice(label: string, publicKey?: string): TunnelNewDeviceDto {
		const { device, privateKey } = this.keys.addDevice(label, publicKey)
		this.logger.info(
			{ label: device.label, fingerprint: device.fingerprint, generated: privateKey !== undefined },
			"Paired a new device for remote access",
		)
		const { host } = parseRelayAddress(this.relay)
		return {
			device: toDeviceDto(device),
			privateKey,
			publicKey: device.publicKey,
			connection: {
				host,
				port: this.port,
				username: "simplex",
				hostFingerprint: this.hostKey.fingerprint,
				localForward: `${LOCAL_FORWARD_PORT}:127.0.0.1:${this.opts.uiTarget().port}`,
			},
		}
	}

	removeDevice(fingerprint: string): boolean {
		const removed = this.keys.removeDevice(fingerprint)
		if (removed) this.logger.warn({ fingerprint }, "Revoked a remote-access device")
		return removed
	}

	private connect(): void {
		if (this.stopped || this.client) return
		let target: { host: string; port: number }
		try {
			target = parseRelayAddress(this.relay)
		} catch (err) {
			this.state = "error"
			this.lastError = err instanceof Error ? err.message : String(err)
			return
		}
		this.state = this.attempt === 0 ? "connecting" : "reconnecting"
		this.pinMismatch = false
		const client = new SshClient()
		this.client = client
		const relay = this.relay

		client.on("ready", () => {
			client.forwardIn("0.0.0.0", 0, (err, port) => {
				if (err) {
					this.lastError = `Relay refused the port forward: ${err.message}`
					this.logger.warn({ relay, err }, "Relay refused the port forward")
					client.end()
					return
				}
				this.attempt = 0
				this.port = port
				this.connectedAt = Date.now()
				this.lastError = undefined
				this.state = "connected"
				this.logger.info({ relay, port }, "Remote access tunnel is up")
			})
		})

		client.on("tcp connection", (details, accept, reject) => {
			if (this.stopped) return reject()
			const channel: ClientChannel = accept()
			this.server.inject(channel, { ip: details.srcIP, port: details.srcPort })
		})

		client.on("error", (err) => {
			if (!this.pinMismatch) this.lastError = err.message
			this.logger.warn({ relay, err: err.message }, "Tunnel connection error")
		})

		client.on("close", () => {
			if (this.client !== client) return
			this.client = undefined
			this.port = undefined
			this.connectedAt = undefined
			if (this.stopped || !this.enabled) {
				this.state = this.enabled ? "disconnected" : "disabled"
				return
			}
			this.scheduleReconnect()
		})

		client.connect({
			host: target.host,
			port: target.port,
			username: "simplex",
			privateKey: this.operatorKey.privateKey,
			keepaliveInterval: 15_000,
			keepaliveCountMax: 3,
			readyTimeout: 30_000,
			hostVerifier: (key: Buffer) => this.verifyRelay(relay, key),
		})
	}

	/**
	 * Pins the relay: a configured fingerprint wins; otherwise the key seen on
	 * first contact with this relay address is remembered and enforced after.
	 * A mismatch is refused loudly rather than retried quietly.
	 */
	private verifyRelay(relay: string, key: Buffer): boolean {
		const seen = fingerprintOf(key)
		const expected =
			this.config.relayHostKey?.trim() ||
			(this.keys.knownRelay()?.relay === relay ? this.keys.knownRelay()?.fingerprint : undefined)
		if (expected && expected !== seen) {
			this.pinMismatch = true
			this.lastError = `Relay host key mismatch: expected ${expected}, got ${seen}. Set [simplex.tunnel] relayHostKey or delete tunnel/known_relay if the relay was rebuilt.`
			this.logger.error({ relay, expected, seen }, "Relay host key mismatch, refusing to connect")
			return false
		}
		if (!expected) {
			this.keys.rememberRelay(relay, seen)
			this.logger.info({ relay, fingerprint: seen }, "Pinned the relay host key on first contact")
		}
		this.relayFingerprint = seen
		return true
	}

	private scheduleReconnect(): void {
		this.state = "reconnecting"
		const delay = Math.min(1000 * 2 ** Math.min(this.attempt, 10), this.maxBackoffMs)
		this.attempt++
		this.logger.info({ delayMs: delay, attempt: this.attempt }, "Tunnel down, reconnecting")
		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = undefined
			this.connect()
		}, delay)
	}

	private clearReconnect(): void {
		if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
		this.reconnectTimer = undefined
	}

	private disconnect(): void {
		const client = this.client
		this.client = undefined
		this.port = undefined
		this.connectedAt = undefined
		client?.end()
	}
}

function toDeviceDto(device: { fingerprint: string; label: string; addedAt: number }): TunnelDeviceDto {
	return { fingerprint: device.fingerprint, label: device.label, addedAt: device.addedAt }
}
