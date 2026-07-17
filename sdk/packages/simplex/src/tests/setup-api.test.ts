import { describe, it, expect, afterEach, vi } from "vitest"
import { mkdtempSync, readFileSync, statSync, existsSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { parse } from "toml"
import { UiServer, type SetupContext } from "@/services/server/UiServer"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"
import { deriveSubstrateKeyPair } from "@/services/substrate-key"
import { startMockRpc, type MockRpc } from "./helpers/mock-rpc"

const CSRF = { "Content-Type": "application/json", "X-Simplex-UI": "1" }
const TEST_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
const TEST_ADDRESS = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"

describe("setup API", () => {
	let server: UiServer | undefined
	let rpc: MockRpc | undefined

	afterEach(() => {
		server?.stop()
		server = undefined
		rpc?.close()
		rpc = undefined
	})

	async function startInitServer(setupOverrides: Partial<SetupContext> = {}) {
		const dir = mkdtempSync(join(tmpdir(), "simplex-setup-"))
		const configPath = join(dir, "filler-config.toml")
		const onSaveAndStart = vi.fn().mockResolvedValue(undefined)
		server = new UiServer({
			mode: "init",
			setup: { configPath, onSaveAndStart, ...setupOverrides },
		})
		const port = await server.start(0)
		return { base: `http://127.0.0.1:${port}`, configPath, onSaveAndStart }
	}

	function post(base: string, endpoint: string, body: unknown) {
		return fetch(`${base}/api/setup/${endpoint}`, {
			method: "POST",
			headers: CSRF,
			body: JSON.stringify(body),
		})
	}

	function minimalConfig(rpcUrl: string): FillerTomlConfig {
		return {
			simplex: {
				signer: { type: SignerType.PrivateKey, key: TEST_KEY },
				maxConcurrentOrders: 5,
				queue: { maxRechecks: 10, recheckDelayMs: 30000 },
				substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
				hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
			},
			strategies: [
				{
					type: "stable",
					bpsCurve: [
						{ amount: "100", value: 100 },
						{ amount: "100000", value: 10 },
					],
				},
			],
			chains: [{ rpcUrls: [rpcUrl], bundlerUrl: "https://api.pimlico.io/v2/1/rpc?apikey=secretpimlicokey" }],
		}
	}

	it("serves wizard defaults", async () => {
		const { base } = await startInitServer()
		const res = await fetch(`${base}/api/setup/defaults`)
		const body = await res.json()
		expect(body.chains.length).toBeGreaterThan(5)
		expect(body.hyperbridgeWs.mainnet).toContain("wss://")
		expect(body.stableBpsCurve.length).toBeGreaterThanOrEqual(2)
		expect(body.maxConcurrentOrders).toBe(5)
	})

	it("validates an RPC against the expected chain id", async () => {
		rpc = await startMockRpc({ chainId: 8453 })
		const { base } = await startInitServer()

		const ok = await (await post(base, "validate-rpc", { url: rpc.url, expectedChainId: 8453 })).json()
		expect(ok).toEqual({ ok: true, results: [{ url: rpc.url, chainId: 8453 }] })

		const mismatch = await (await post(base, "validate-rpc", { url: rpc.url, expectedChainId: 42161 })).json()
		expect(mismatch.ok).toBe(false)
		expect(mismatch.results[0].error).toContain("expected 42161")
	})

	it("rejects quorum URLs sharing a hostname", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const res = await (
			await post(base, "validate-rpc", { urls: [rpc.url, rpc.url], expectedChainId: 1 })
		).json()
		expect(res.ok).toBe(false)
		expect(res.error).toContain("different domains")
	})

	it("probes bundlers as a warning-only check", async () => {
		rpc = await startMockRpc({})
		const { base } = await startInitServer()

		const ok = await (await post(base, "validate-bundler", { url: rpc.url })).json()
		expect(ok.ok).toBe(true)
		expect(ok.entryPoints).toHaveLength(1)

		const dead = await (await post(base, "validate-bundler", { url: "http://127.0.0.1:1/rpc" })).json()
		expect(dead.ok).toBe(true)
		expect(dead.warning).toBeDefined()
	})

	it("validates ERC-20 tokens on-chain", async () => {
		rpc = await startMockRpc({ symbol: "cNGN", decimals: 6 })
		const { base } = await startInitServer()

		const ok = await (
			await post(base, "validate-token", { rpcUrl: rpc.url, address: "0x1111111111111111111111111111111111111111" })
		).json()
		expect(ok).toEqual({ ok: true, symbol: "cNGN", decimals: 6 })

		const bad = await (await post(base, "validate-token", { rpcUrl: rpc.url, address: "nope" })).json()
		expect(bad.ok).toBe(false)
	})

	it("reports missing bytecode for empty addresses", async () => {
		rpc = await startMockRpc({ code: "0x" })
		const { base } = await startInitServer()
		const res = await (
			await post(base, "validate-token", { rpcUrl: rpc.url, address: "0x1111111111111111111111111111111111111111" })
		).json()
		expect(res).toEqual({ ok: false, error: "No contract deployed at this address" })
	})

	it("derives the EVM address for a private key", async () => {
		const { base } = await startInitServer()
		const res = await (await post(base, "derive-evm-address", { privateKey: TEST_KEY })).json()
		expect(res.address).toBe(TEST_ADDRESS)

		const bad = await post(base, "derive-evm-address", { privateKey: "0x123" })
		expect(bad.status).toBe(400)
	})

	it("generates a substrate key whose address matches re-derivation", async () => {
		const { base } = await startInitServer()
		const res = await (await post(base, "generate-substrate-key", {})).json()
		expect(res.mnemonic.split(" ")).toHaveLength(12)
		const pair = await deriveSubstrateKeyPair(res.mnemonic)
		expect(pair.address).toBe(res.address)

		const pasted = await (await post(base, "generate-substrate-key", { key: res.mnemonic })).json()
		expect(pasted).toEqual({ address: res.address })
	})

	it("previews a masked TOML without leaking secrets", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const config = minimalConfig(rpc.url)

		const res = await (await post(base, "preview", { config })).json()
		expect(res.ok).toBe(true)
		expect(res.toml).not.toContain(TEST_KEY)
		expect(res.toml).not.toContain("basket hold race")
		expect(res.toml).not.toContain("secretpimlicokey")
		expect(res.toml).toContain("[simplex.signer]")
	})

	it("rejects an invalid config at preview with the validation message", async () => {
		const { base } = await startInitServer()
		const config = minimalConfig("http://127.0.0.1:1")
		config.strategies = []
		const res = await post(base, "preview", { config })
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("At least one strategy")
	})

	it("save-and-start writes the config 0600, calls the boot callback and flips to operator", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base, configPath, onSaveAndStart } = await startInitServer()
		const config = minimalConfig(rpc.url)

		const res = await post(base, "save-and-start", { config })
		expect(res.status).toBe(202)
		expect((await res.json()).configPath).toBe(configPath)

		expect(existsSync(configPath)).toBe(true)
		expect(statSync(configPath).mode & 0o777).toBe(0o600)
		const written = parse(readFileSync(configPath, "utf-8")) as FillerTomlConfig
		expect(() => validateConfig(written)).not.toThrow()
		expect(written.simplex.signer?.type).toBe("privateKey")
		expect(JSON.parse(JSON.stringify(written))).toEqual(JSON.parse(JSON.stringify(config)))

		await vi.waitFor(() => expect(onSaveAndStart).toHaveBeenCalledTimes(1))
		const [bootedConfig, toml, path] = onSaveAndStart.mock.calls[0]
		expect(path).toBe(configPath)
		expect(toml).toContain("[[strategies]]")
		expect(JSON.parse(JSON.stringify(bootedConfig))).toEqual(JSON.parse(JSON.stringify(config)))
	})

	it("rejects invalid configs before writing anything", async () => {
		const { base, configPath, onSaveAndStart } = await startInitServer()
		const config = minimalConfig("http://127.0.0.1:1")
		config.simplex.substratePrivateKey = ""

		const res = await post(base, "save-and-start", { config })
		expect(res.status).toBe(400)
		expect(existsSync(configPath)).toBe(false)
		expect(onSaveAndStart).not.toHaveBeenCalled()
	})

	it("reports failed boots via start-status and stays in init mode", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const onSaveAndStart = vi.fn().mockRejectedValue(new Error("delegation exploded"))
		const { base } = await startInitServer({ onSaveAndStart })

		const res = await post(base, "save-and-start", { config: minimalConfig(rpc.url) })
		expect(res.status).toBe(202)

		await vi.waitFor(async () => {
			const status = await (await fetch(`${base}/api/setup/start-status`)).json()
			expect(status).toEqual({ state: "failed", error: "delegation exploded" })
		})
		expect((await (await fetch(`${base}/api/status`)).json()).mode).toBe("init")
	})

	it("guards save-and-start against concurrent starts", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		let resolveBoot!: () => void
		const onSaveAndStart = vi.fn().mockImplementation(
			() =>
				new Promise<void>((resolve) => {
					resolveBoot = resolve
				}),
		)
		const { base } = await startInitServer({ onSaveAndStart })
		const config = minimalConfig(rpc.url)

		expect((await post(base, "save-and-start", { config })).status).toBe(202)
		expect((await post(base, "save-and-start", { config })).status).toBe(409)
		resolveBoot()
	})
})
