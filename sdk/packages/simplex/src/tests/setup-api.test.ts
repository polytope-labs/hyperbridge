import { describe, it, expect, afterEach, vi } from "vitest"
import { mkdtempSync, readFileSync, statSync, existsSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { parse } from "toml"
import { UiServer, type SetupContext } from "@/services/server/UiServer"
import { validateConfig, type FillerConfigFile } from "@/config/filler-toml"
import { SignerType, signerFromToml } from "@/services/wallet"
import { SECRET_PHRASE_MASK } from "@/services/server/setup-api"
import { FillerConfigService } from "@/services/FillerConfigService"
import { deriveSubstrateKeyPair } from "@/services/substrate-key"
import { startMockRpc, type MockRpc } from "./helpers/mock-rpc"
import { encryptedConfigStore, isEncryptedConfig } from "@/config/storage"

const CSRF = { "Content-Type": "application/json", "X-Simplex-UI": "1" }
const TEST_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
const TEST_ADDRESS = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
const TEST_PHRASE = "test test test test test test test test test test test junk"
const TEST_PHRASE_ADDRESS = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
// Words no error message or TOML comment uses, so finding one in a response means the phrase leaked.
const DISTINCT_PHRASE = "zebra walnut giraffe umbrella quantum lizard oyster pumpkin volcano kangaroo jaguar tomato"

function expectNoPhraseWords(text: string, phrase: string) {
	const lower = text.toLowerCase()
	for (const word of phrase.toLowerCase().split(/\s+/).filter(Boolean)) {
		expect(lower).not.toContain(word)
	}
}

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

	function minimalConfig(rpcUrl: string): FillerConfigFile {
		return {
			simplex: {
				signer: { type: SignerType.PrivateKey, key: TEST_KEY },
				maxConcurrentOrders: 5,
				substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
				hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
			},
			pairs: [
				{
					token0: "USDC",
					token1: "USDC",
				},
			],
			chains: [{ rpcUrls: [rpcUrl], bundlerUrl: "https://api.pimlico.io/v2/1/rpc?apikey=secretpimlicokey" }],
			orderbook: { url: "https://orderbook.example/graphql" },
		}
	}

	it("serves mainnet-only wizard defaults", async () => {
		const { base } = await startInitServer()
		const res = await fetch(`${base}/api/setup/defaults`)
		expect(res.status).toBe(200)
		const defaults = await res.json()
		expect(defaults.chains).toHaveLength(5)
		expect(defaults.chains.every((chain: { network: string }) => chain.network === "mainnet")).toBe(true)
		expect(defaults.hyperbridgeWs).toEqual({ mainnet: "wss://nexus.rpc.polytope.technology" })
		expect(defaults).not.toHaveProperty("testnetConfirmationPoints")
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

	it("uses mainnet Alchemy endpoints even when a client requests testnet", async () => {
		const fetchChainId = vi.fn(async () => 1)
		const { base } = await startInitServer({ deps: { fetchChainId } })

		const response = await (await post(base, "validate-alchemy-key", { apiKey: "key", network: "testnet" })).json()
		expect(response.valid).toBe(true)
		expect(response.chains.map((chain: { chainId: number }) => chain.chainId)).toEqual([1, 42161, 8453, 137, 56])
		expect(fetchChainId).toHaveBeenCalledWith(expect.stringContaining("eth-mainnet"))
	})

	it("rejects quorum URLs sharing a hostname", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const res = await (await post(base, "validate-rpc", { urls: [rpc.url, rpc.url], expectedChainId: 1 })).json()
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

	it("warns when the bundler does not list the chain's EntryPoint", async () => {
		rpc = await startMockRpc({})
		const { base } = await startInitServer()
		const required = new FillerConfigService([]).getEntryPointAddress("EVM-8453")!

		const res = await (await post(base, "validate-bundler", { url: rpc.url, chainId: 8453 })).json()
		expect(res.ok).toBe(true)
		expect(res.warning).toContain(required)
		expect(res.warning).toContain("0x0000000071727De22E5E9d8BAf0edAc6f37da032")
	})

	it("accepts a bundler listing the chain's EntryPoint in any case, and skips chains without one", async () => {
		const required = new FillerConfigService([]).getEntryPointAddress("EVM-8453")!
		const rpcRequest = vi.fn(async () => [required.toLowerCase()])
		const { base } = await startInitServer({ deps: { rpcRequest } })

		const url = "https://bundler.example"
		const listed = await (await post(base, "validate-bundler", { url, chainId: 8453 })).json()
		expect(listed).toEqual({ ok: true, entryPoints: [required.toLowerCase()] })
		expect(rpcRequest).toHaveBeenCalledWith(url, "eth_supportedEntryPoints", [])

		const unknown = await (await post(base, "validate-bundler", { url, chainId: 31337 })).json()
		expect(unknown.warning).toBeUndefined()
	})

	it("warns without promising a refusal when the bundler's answer is not a list of addresses", async () => {
		const required = new FillerConfigService([]).getEntryPointAddress("EVM-8453")!
		const rpcRequest = vi.fn(async () => ({ entryPoints: [required] }))
		const { base } = await startInitServer({ deps: { rpcRequest } })

		const res = await (await post(base, "validate-bundler", { url: "https://bundler.example", chainId: 8453 })).json()
		expect(res.ok).toBe(true)
		expect(res.warning).toContain("not a list of addresses")
		expect(res.warning).toContain(required)
		expect(res.warning).not.toContain("refuse")
	})

	it("validates ERC-20 tokens on-chain", async () => {
		rpc = await startMockRpc({ symbol: "cNGN", decimals: 6 })
		const { base } = await startInitServer()

		const ok = await (
			await post(base, "validate-token", {
				rpcUrl: rpc.url,
				address: "0x1111111111111111111111111111111111111111",
			})
		).json()
		expect(ok).toEqual({ ok: true, symbol: "cNGN", decimals: 6 })

		const bad = await (await post(base, "validate-token", { rpcUrl: rpc.url, address: "nope" })).json()
		expect(bad.ok).toBe(false)
	})

	it("reports missing bytecode for empty addresses", async () => {
		rpc = await startMockRpc({ code: "0x" })
		const { base } = await startInitServer()
		const res = await (
			await post(base, "validate-token", {
				rpcUrl: rpc.url,
				address: "0x1111111111111111111111111111111111111111",
			})
		).json()
		expect(res).toEqual({ ok: false, error: "No contract deployed at this address" })
	})

	it("derives the EVM address for a private key, with or without the 0x prefix", async () => {
		const { base } = await startInitServer()
		const res = await (await post(base, "derive-evm-address", { privateKey: TEST_KEY })).json()
		expect(res.address).toBe(TEST_ADDRESS)

		const bare = await (await post(base, "derive-evm-address", { privateKey: TEST_KEY.slice(2) })).json()
		expect(bare.address).toBe(TEST_ADDRESS)

		const bad = await post(base, "derive-evm-address", { privateKey: "0x123" })
		expect(bad.status).toBe(400)
	})

	it("derives the EVM address for a secret phrase at the requested account index", async () => {
		const { base } = await startInitServer()
		const first = await post(base, "derive-evm-address", { phrase: TEST_PHRASE })
		expect(first.status).toBe(200)
		expect(await first.json()).toEqual({ address: TEST_PHRASE_ADDRESS })

		const second = await post(base, "derive-evm-address", { phrase: TEST_PHRASE, accountIndex: 1 })
		expect(second.status).toBe(200)
		expect(await second.json()).toEqual({ address: TEST_ADDRESS })

		const untidy = await post(base, "derive-evm-address", { phrase: `  ${TEST_PHRASE.toUpperCase()}\n` })
		expect(await untidy.json()).toEqual({ address: TEST_PHRASE_ADDRESS })
	})

	it("rejects a bad secret phrase request without echoing any of its words", async () => {
		const { base } = await startInitServer()
		const words = DISTINCT_PHRASE.split(" ")
		const badChecksum = [...words.slice(0, -1), "hedgehog"].join(" ")
		const cases: Array<{ body: Record<string, unknown>; phrase: string; error: string }> = [
			{ body: { phrase: badChecksum }, phrase: badChecksum, error: "checksum" },
			{ body: { phrase: words.slice(0, 11).join(" ") }, phrase: DISTINCT_PHRASE, error: "got 11" },
			{
				body: { phrase: [...words.slice(0, -1), "xylophonist"].join(" ") },
				phrase: `${DISTINCT_PHRASE} xylophonist`,
				error: "wordlist",
			},
			{ body: { phrase: DISTINCT_PHRASE, accountIndex: -1 }, phrase: DISTINCT_PHRASE, error: "accountIndex" },
			{ body: { phrase: DISTINCT_PHRASE, accountIndex: 1.5 }, phrase: DISTINCT_PHRASE, error: "accountIndex" },
			{
				body: { phrase: DISTINCT_PHRASE, accountIndex: DISTINCT_PHRASE },
				phrase: DISTINCT_PHRASE,
				error: "accountIndex",
			},
			{ body: { phrase: DISTINCT_PHRASE, privateKey: TEST_KEY }, phrase: DISTINCT_PHRASE, error: "not both" },
			{ body: { phrase: words }, phrase: DISTINCT_PHRASE, error: "must be a string" },
		]

		for (const { body, phrase, error } of cases) {
			const res = await post(base, "derive-evm-address", body)
			expect(res.status).toBe(400)
			const text = await res.text()
			expect(JSON.parse(text).error).toContain(error)
			expectNoPhraseWords(text, phrase)
			expect(text).not.toContain(TEST_KEY)
		}
	})

	it("does not echo a malformed request body", async () => {
		const { base } = await startInitServer()
		const res = await fetch(`${base}/api/setup/derive-evm-address`, {
			method: "POST",
			headers: CSRF,
			body: `{"phrase": "${DISTINCT_PHRASE}`,
		})
		expect(res.status).toBe(400)
		const text = await res.text()
		expect(JSON.parse(text)).toEqual({ error: "Invalid JSON body" })
		expectNoPhraseWords(text, DISTINCT_PHRASE)
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

	it("previews a secret phrase signer with the whole phrase masked", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const config = minimalConfig(rpc.url)
		config.simplex.signer = { type: SignerType.SecretPhrase, phrase: DISTINCT_PHRASE, accountIndex: 3 }

		const res = await post(base, "preview", { config })
		expect(res.status).toBe(200)
		const text = await res.text()
		expectNoPhraseWords(text, DISTINCT_PHRASE)
		const { toml } = JSON.parse(text)
		expect(parse(toml).simplex.signer).toEqual({
			type: "secretPhrase",
			phrase: SECRET_PHRASE_MASK,
			accountIndex: 3,
		})
	})

	it("masks every secret phrase to the same placeholder, whatever its length", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const phrases = [
			TEST_PHRASE,
			"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
		]
		for (const phrase of phrases) {
			const config = minimalConfig(rpc.url)
			config.simplex.signer = { type: SignerType.SecretPhrase, phrase }
			const { toml } = await (await post(base, "preview", { config })).json()
			expect(parse(toml).simplex.signer.phrase).toBe("****")
		}
	})

	it("rejects an invalid secret phrase signer at preview and save without echoing it", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base, configPath, onSaveAndStart } = await startInitServer()
		const words = DISTINCT_PHRASE.split(" ")
		const badChecksum = [...words.slice(0, -1), "hedgehog"].join(" ")
		const signers = [
			{ signer: { type: SignerType.SecretPhrase, phrase: badChecksum }, error: "checksum" },
			{ signer: { type: SignerType.SecretPhrase, phrase: words.slice(0, 11).join(" ") }, error: "got 11" },
			{
				signer: { type: SignerType.SecretPhrase, phrase: DISTINCT_PHRASE, accountIndex: -1 },
				error: "accountIndex",
			},
			{ signer: { type: SignerType.SecretPhrase, phrase: SECRET_PHRASE_MASK }, error: "got 1" },
		]

		for (const { signer, error } of signers) {
			for (const endpoint of ["preview", "save-and-start"]) {
				const config = { ...minimalConfig(rpc.url), simplex: { ...minimalConfig(rpc.url).simplex, signer } }
				const res = await post(base, endpoint, { config })
				expect(res.status).toBe(400)
				const text = await res.text()
				expect(JSON.parse(text).error).toContain(error)
				expectNoPhraseWords(text, `${DISTINCT_PHRASE} hedgehog`)
			}
		}
		expect(existsSync(configPath)).toBe(false)
		expect(onSaveAndStart).not.toHaveBeenCalled()
	})

	it("rejects a phrase pasted into accountIndex at preview and save without echoing it", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base, configPath, onSaveAndStart } = await startInitServer()
		const signer = { type: SignerType.SecretPhrase, phrase: TEST_PHRASE, accountIndex: DISTINCT_PHRASE }

		for (const endpoint of ["preview", "save-and-start"]) {
			const config = { ...minimalConfig(rpc.url), simplex: { ...minimalConfig(rpc.url).simplex, signer } }
			const res = await post(base, endpoint, { config })
			expect(res.status).toBe(400)
			const text = await res.text()
			expect(JSON.parse(text).error).toContain("accountIndex must be an integer")
			expectNoPhraseWords(text, DISTINCT_PHRASE)
		}
		expect(existsSync(configPath)).toBe(false)
		expect(onSaveAndStart).not.toHaveBeenCalled()
	})

	it("rejects an invalid config at preview with the validation message", async () => {
		const { base } = await startInitServer()
		const config = minimalConfig("http://127.0.0.1:1")
		config.chains = []
		const res = await post(base, "preview", { config })
		expect(res.status).toBe(400)
		expect((await res.json()).error).toContain("At least one chain")
	})

	it("rejects testnet chains at preview", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base } = await startInitServer()
		const config = minimalConfig(rpc.url)
		const response = await post(base, "preview", { config, chainIds: [11155111] })
		expect(response.status).toBe(400)
		expect((await response.json()).error).toContain("mainnet")
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
		const written = parse(readFileSync(configPath, "utf-8")) as FillerConfigFile
		expect(() => validateConfig(written)).not.toThrow()
		expect(written.simplex.signer?.type).toBe("privateKey")
		expect(JSON.parse(JSON.stringify(written))).toEqual(JSON.parse(JSON.stringify(config)))

		await vi.waitFor(() => expect(onSaveAndStart).toHaveBeenCalledTimes(1))
		const [bootedConfig, toml, path] = onSaveAndStart.mock.calls[0]
		expect(path).toBe(configPath)
		expect(toml).toContain("[[pairs]]")
		expect(JSON.parse(JSON.stringify(bootedConfig))).toEqual(JSON.parse(JSON.stringify(config)))
	})

	it("save-and-start writes a normalised secret phrase that loads back to its wallet", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base, configPath, onSaveAndStart } = await startInitServer()
		const config = minimalConfig(rpc.url)
		config.simplex.signer = {
			type: SignerType.SecretPhrase,
			phrase: `  ${TEST_PHRASE.toUpperCase().split(" ").join("  ")}\n`,
			accountIndex: 1,
		}

		const res = await post(base, "save-and-start", { config })
		expect(res.status).toBe(202)
		expect(await res.text()).not.toContain("junk")

		expect(statSync(configPath).mode & 0o777).toBe(0o600)
		const written = parse(readFileSync(configPath, "utf-8")) as FillerConfigFile
		expect(written.simplex.signer).toEqual({ type: "secretPhrase", phrase: TEST_PHRASE, accountIndex: 1 })
		expect((await signerFromToml(written.simplex.signer))?.address).toBe(TEST_ADDRESS)

		await vi.waitFor(() => expect(onSaveAndStart).toHaveBeenCalledTimes(1))
		const [bootedConfig] = onSaveAndStart.mock.calls[0]
		expect(bootedConfig.simplex.signer.phrase).toBe(TEST_PHRASE)
	})

	it("encrypts initial setup through the desktop writer without changing the in-memory boot config", async () => {
		const key = Buffer.alloc(32, 7)
		const { base, configPath, onSaveAndStart } = await startInitServer({
			writeConfigFile: (path, content) => encryptedConfigStore(path, key).write(path, content),
		})
		const res = await post(base, "save-and-start", { config: minimalConfig("http://127.0.0.1:9") })
		expect(res.status).toBe(202)
		const ciphertext = readFileSync(configPath, "utf8")
		expect(isEncryptedConfig(ciphertext)).toBe(true)
		expect(ciphertext).not.toContain(TEST_KEY)
		expect(encryptedConfigStore(configPath, key).read()).toContain(TEST_KEY)
		await vi.waitFor(() => expect(onSaveAndStart).toHaveBeenCalledTimes(1))
		expect(onSaveAndStart.mock.calls[0][1]).toContain(TEST_KEY)
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
		expect(await (await fetch(`${base}/health`)).json()).toEqual({
			status: "starting",
			mode: "init",
			pid: process.pid,
		})
		expect((await post(base, "save-and-start", { config })).status).toBe(409)
		resolveBoot()
	})

	it("rejects save-and-start after graceful shutdown begins", async () => {
		rpc = await startMockRpc({ chainId: 1 })
		const { base, onSaveAndStart } = await startInitServer()
		server!.beginStopping()

		const response = await post(base, "save-and-start", { config: minimalConfig(rpc.url) })
		expect(response.status).toBe(409)
		expect(await response.json()).toEqual({ error: "Simplex is stopping" })
		expect(onSaveAndStart).not.toHaveBeenCalled()
	})
})
