import { describe, it, expect, afterEach, vi } from "vitest"
import { mkdtempSync, readFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { parse } from "toml"
import { UiServer, type OperatorContext } from "@/services/server/UiServer"
import type { TunnelControls } from "@/services/tunnel/TunnelService"
import type { TunnelStatusDto } from "@/services/server/dto"
import type { FillerConfigFile } from "@/config/filler-toml"
import { ActivityRecorder } from "@/data/recorder"
import { MemoryDataStore } from "@/data/memory"
import { SignerType } from "@/services/wallet"

const CSRF = { "X-Simplex-UI": "1", "Content-Type": "application/json" }

/** An in-memory tunnel: records what the UI asked for, no sockets. */
function fakeTunnel(): TunnelControls & { calls: unknown[]; enabled: boolean; relay: string; devices: string[] } {
	const state = {
		calls: [] as unknown[],
		enabled: false,
		relay: "simplex.tunnel.polytope.technology:443",
		devices: [] as string[],
	}
	const status = (): TunnelStatusDto => ({
		enabled: state.enabled,
		state: state.enabled ? "connected" : "disabled",
		relay: state.relay,
		port: state.enabled ? 24567 : undefined,
		hostFingerprint: "SHA256:host",
		operatorFingerprint: "SHA256:operator",
		devices: state.devices.map((label, i) => ({ fingerprint: `SHA256:dev${i}`, label, addedAt: 1 })),
		activeConnections: 0,
		connection: {
			host: state.relay.split(":")[0],
			port: state.enabled ? 24567 : undefined,
			username: "simplex",
			hostFingerprint: "SHA256:host",
			localForward: "8686:127.0.0.1:8686",
		},
	})
	return Object.assign(state, {
		status,
		configure: vi.fn(async (update: { enabled?: boolean; relay?: string }) => {
			state.calls.push(update)
			if (update.enabled !== undefined) state.enabled = update.enabled
			if (update.relay !== undefined) state.relay = update.relay
		}),
		addDevice: vi.fn((label: string, publicKey?: string) => {
			if (publicKey === "bad") throw new Error("Not a valid SSH public key")
			state.devices.push(label)
			const device = status().devices.at(-1)!
			return {
				device,
				privateKey:
					publicKey === undefined
						? "-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----\n"
						: undefined,
				publicKey: publicKey ?? "ssh-ed25519 AAAA",
				connection: status().connection,
			}
		}),
		removeDevice: vi.fn((fingerprint: string) => {
			const index = Number(fingerprint.replace("SHA256:dev", ""))
			if (Number.isNaN(index) || index >= state.devices.length) return false
			state.devices.splice(index, 1)
			return true
		}),
	})
}

function operatorContext(tunnel?: TunnelControls): OperatorContext & { configPath: string } {
	const dataDir = mkdtempSync(join(tmpdir(), "simplex-ui-tunnel-"))
	const data = new MemoryDataStore()
	const config: FillerConfigFile = {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0xab" },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example",
		},
		pairs: [],
		chains: [],
	}
	return {
		strategies: [],
		filler: { pause() {}, resume() {}, isPaused: () => false, getWatchOnly: () => ({}) },
		balances: { getSnapshot: () => ({ updatedAt: null, status: "loading", chains: [], issues: [] }) },
		haltControls: [],
		config,
		stop: vi.fn().mockResolvedValue(undefined),
		activity: new ActivityRecorder(data.activity),
		bids: data.bids,
		setPaused: vi.fn(),
		setLogLevel: vi.fn(),
		applyAllowlist: vi.fn(),
		applyRebalancing: vi.fn(),
		tunnel,
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(dataDir, "filler-config.toml"),
		chains: [],
		strategyTypes: [],
	}
}

describe("UiServer remote-access routes", () => {
	let server: UiServer | undefined
	afterEach(() => {
		server?.stop()
		server = undefined
	})

	async function start(tunnel?: TunnelControls) {
		const operator = operatorContext(tunnel)
		server = new UiServer({ mode: "operator", operator })
		const port = await server.start(0)
		return { base: `http://127.0.0.1:${port}`, operator }
	}

	it("reports 404 when the filler has no tunnel and hides it from the config summary", async () => {
		const { base } = await start()
		const res = await fetch(`${base}/api/tunnel`)
		expect(res.status).toBe(404)
		const config = (await (await fetch(`${base}/api/config`)).json()) as { tunnel?: unknown }
		expect(config.tunnel).toBeUndefined()
	})

	it("serves status, persists enable/relay changes, and applies them live", async () => {
		const tunnel = fakeTunnel()
		const { base, operator } = await start(tunnel)
		expect(await (await fetch(`${base}/api/tunnel`)).json()).toMatchObject({ enabled: false, state: "disabled" })

		const enabled = await fetch(`${base}/api/tunnel`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ enabled: true }),
		})
		expect(enabled.status).toBe(200)
		expect(await enabled.json()).toMatchObject({ enabled: true, state: "connected", port: 24567, persisted: true })
		expect(tunnel.calls).toEqual([{ enabled: true, relay: undefined }])
		const written = parse(readFileSync(operator.configPath, "utf8")) as FillerConfigFile
		expect(written.simplex.tunnel).toEqual({ enabled: true })

		const relay = await fetch(`${base}/api/tunnel`, {
			method: "PUT",
			headers: CSRF,
			body: JSON.stringify({ relay: " relay.example.com:2222 " }),
		})
		expect(relay.status).toBe(200)
		expect(tunnel.relay).toBe("relay.example.com:2222")
		const rewritten = parse(readFileSync(operator.configPath, "utf8")) as FillerConfigFile
		expect(rewritten.simplex.tunnel).toEqual({ enabled: true, relay: "relay.example.com:2222" })

		const config = (await (await fetch(`${base}/api/config`)).json()) as { tunnel?: unknown }
		expect(config.tunnel).toEqual({ enabled: true, devices: 0 })
	})

	it("validates the update body before touching anything", async () => {
		const tunnel = fakeTunnel()
		const { base } = await start(tunnel)
		for (const body of [{ enabled: "yes" }, { relay: 12 }, { relay: "host:70000" }, { relay: "" }]) {
			const res = await fetch(`${base}/api/tunnel`, { method: "PUT", headers: CSRF, body: JSON.stringify(body) })
			expect(res.status, JSON.stringify(body)).toBe(400)
		}
		const noCsrf = await fetch(`${base}/api/tunnel`, {
			method: "PUT",
			headers: { "Content-Type": "application/json" },
			body: "{}",
		})
		expect(noCsrf.status).toBe(403)
		expect(tunnel.calls).toEqual([])
	})

	it("pairs and revokes devices", async () => {
		const tunnel = fakeTunnel()
		const { base } = await start(tunnel)
		const missing = await fetch(`${base}/api/tunnel/devices`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ label: "  " }),
		})
		expect(missing.status).toBe(400)

		const paired = await fetch(`${base}/api/tunnel/devices`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ label: "phone" }),
		})
		expect(paired.status).toBe(201)
		const body = (await paired.json()) as { device: { label: string; fingerprint: string }; privateKey: string }
		expect(body.device.label).toBe("phone")
		expect(body.privateKey).toContain("OPENSSH PRIVATE KEY")
		expect((await (await fetch(`${base}/api/tunnel`)).json()).devices).toHaveLength(1)

		const revoked = await fetch(`${base}/api/tunnel/devices/revoke`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ fingerprint: body.device.fingerprint }),
		})
		expect(revoked.status).toBe(200)
		const again = await fetch(`${base}/api/tunnel/devices/revoke`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ fingerprint: body.device.fingerprint }),
		})
		expect(again.status).toBe(404)
		expect((await (await fetch(`${base}/api/tunnel`)).json()).devices).toHaveLength(0)

		// Pasting a public key returns no private key. (The fake indexes devices by
		// position, so this runs after the revoke checks.)
		const pasted = await fetch(`${base}/api/tunnel/devices`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ label: "own", publicKey: "ssh-ed25519 AAAAown" }),
		})
		expect(pasted.status).toBe(201)
		const pastedBody = (await pasted.json()) as { privateKey?: string; publicKey: string }
		expect(pastedBody.privateKey).toBeUndefined()
		expect(pastedBody.publicKey).toBe("ssh-ed25519 AAAAown")
		expect(tunnel.addDevice).toHaveBeenLastCalledWith("own", "ssh-ed25519 AAAAown")
		const wrongType = await fetch(`${base}/api/tunnel/devices`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ label: "x", publicKey: 42 }),
		})
		expect(wrongType.status).toBe(400)
		const invalid = await fetch(`${base}/api/tunnel/devices`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify({ label: "x", publicKey: "bad" }),
		})
		expect(invalid.status).toBe(400)
		expect(((await invalid.json()) as { error: string }).error).toMatch(/Not a valid/)
		expect((await (await fetch(`${base}/api/tunnel`)).json()).devices).toHaveLength(1)
	})
})
