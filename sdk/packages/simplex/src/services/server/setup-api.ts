import type { IncomingMessage, ServerResponse } from "node:http"
import { chmodSync, writeFileSync } from "node:fs"
import { createPublicClient, http, isAddress } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { validateConfig, DEFAULT_CONFIRMATION_POLICIES, type FillerTomlConfig } from "@/config/filler-toml"
import { fetchChainId, validateRpcUrls } from "@/services/FillerConfigService"
import { validateSignerConfig, type SignerConfig } from "@/services/wallet"
import { deriveSubstrateKeyPair, generateSubstrateKey } from "@/services/substrate-key"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { emitFillerToml } from "@/cli/init/emit-toml"
import { chainsForNetwork, HYPERBRIDGE_WS_DEFAULTS, INIT_CHAINS, type InitNetwork } from "@/cli/init/chains"
import { deriveAlchemyRpc } from "@/cli/init/derive/alchemy"
import { maskSecret, withTimeout } from "@/cli/init/prompt-utils"
import {
	DEFAULT_MAX_CONCURRENT_ORDERS,
	DEFAULT_QUEUE,
	DEFAULT_STABLE_BPS_CURVE,
	TESTNET_CONFIRMATION_POINTS,
} from "@/cli/init/state"
import { getLogger } from "../Logger"
import { readBody, sendJson } from "./http-util"
import type { SetupContext, UiServer } from "./UiServer"

const PROBE_TIMEOUT_MS = 10_000

/** Network-facing validators, injectable so tests never hit real providers. */
export interface SetupDeps {
	fetchChainId?: typeof fetchChainId
	rpcRequest?: (url: string, method: string, params: unknown[]) => Promise<unknown>
}

const logger = getLogger("setup")

async function defaultRpcRequest(url: string, method: string, params: unknown[]): Promise<unknown> {
	const response = await fetch(url, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", method, params, id: 1 }),
	})
	if (!response.ok) throw new Error(`HTTP ${response.status}`)
	const json = (await response.json()) as { result?: unknown; error?: { message: string } }
	if (json.error) throw new Error(json.error.message)
	return json.result
}

/**
 * Routes /api/setup/* requests in init mode. Request bodies carry private keys
 * and API tokens — they are never logged and never echoed back unmasked.
 */
export async function handleSetupRequest(
	server: UiServer,
	setup: SetupContext,
	req: IncomingMessage,
	res: ServerResponse,
	path: string,
	method: string,
): Promise<void> {
	const endpoint = path.slice("/api/setup/".length)

	if (endpoint === "defaults") {
		if (method !== "GET") return sendJson(res, 405, { error: "Method not allowed" })
		return sendJson(res, 200, {
			chains: INIT_CHAINS,
			hyperbridgeWs: HYPERBRIDGE_WS_DEFAULTS,
			stableBpsCurve: DEFAULT_STABLE_BPS_CURVE,
			confirmationPolicies: DEFAULT_CONFIRMATION_POLICIES,
			testnetConfirmationPoints: TESTNET_CONFIRMATION_POINTS,
			queue: DEFAULT_QUEUE,
			maxConcurrentOrders: DEFAULT_MAX_CONCURRENT_ORDERS,
			configPath: setup.configPath,
		})
	}

	if (method !== "POST") return sendJson(res, 405, { error: "Method not allowed" })

	let body: Record<string, unknown>
	try {
		const raw = await readBody(req)
		body = raw ? JSON.parse(raw) : {}
	} catch (err) {
		return sendJson(res, 400, { error: err instanceof Error ? err.message : "Invalid JSON body" })
	}

	const deps: Required<SetupDeps> = {
		fetchChainId: setup.deps?.fetchChainId ?? fetchChainId,
		rpcRequest: setup.deps?.rpcRequest ?? defaultRpcRequest,
	}

	try {
		switch (endpoint) {
			case "validate-alchemy-key":
				return sendJson(res, 200, await validateAlchemyKey(body, deps))
			case "validate-rpc":
				return sendJson(res, 200, await validateRpc(body, deps))
			case "validate-bundler":
				return sendJson(res, 200, await validateBundler(body, deps))
			case "validate-token":
				return sendJson(res, 200, await validateToken(body))
			case "derive-evm-address":
				return deriveEvmAddress(body, res)
			case "generate-substrate-key":
				return sendJson(res, 200, await substrateKey(body))
			case "check-substrate-balance":
				return sendJson(res, 200, await checkSubstrateBalance(body))
			case "preview": {
				const result = gateConfig(body)
				if ("error" in result) return sendJson(res, 400, result)
				return sendJson(res, 200, { ok: true, toml: maskToml(result.config, result.chainLabels) })
			}
			case "save-and-start":
				return saveAndStart(server, setup, body, res)
			default:
				return sendJson(res, 404, { error: "Not found" })
		}
	} catch (err) {
		return sendJson(res, 400, { error: err instanceof Error ? err.message : String(err) })
	}
}

async function validateAlchemyKey(body: Record<string, unknown>, deps: Required<SetupDeps>) {
	const apiKey = String(body.apiKey ?? "").trim()
	const network = (body.network === "testnet" ? "testnet" : "mainnet") as InitNetwork
	if (!apiKey) return { valid: false, error: "apiKey is required", chains: [] }

	const chains = chainsForNetwork(network).map((meta) => {
		const rpcUrl = deriveAlchemyRpc(apiKey, meta.chainId)
		return {
			chainId: meta.chainId,
			stateMachineId: meta.stateMachineId,
			label: meta.label,
			note: meta.note,
			rpcUrl,
			// Alchemy serves ERC-4337 bundler methods on the same endpoint.
			bundlerUrl: rpcUrl,
		}
	})

	const probe = chains.find((c) => c.rpcUrl)
	if (!probe) return { valid: false, error: "Alchemy serves none of the selected chains", chains }
	try {
		const chainId = await withTimeout(deps.fetchChainId(probe.rpcUrl!), PROBE_TIMEOUT_MS, "Alchemy key check")
		if (chainId !== probe.chainId) {
			return { valid: false, error: `Key check returned unexpected chainId ${chainId}`, chains }
		}
	} catch (err) {
		return { valid: false, error: err instanceof Error ? err.message : String(err), chains }
	}
	return { valid: true, chains }
}

async function validateRpc(body: Record<string, unknown>, deps: Required<SetupDeps>) {
	const urls = Array.isArray(body.urls) ? body.urls.map(String) : [String(body.url ?? "")]
	const expectedChainId = body.expectedChainId === undefined ? undefined : Number(body.expectedChainId)
	try {
		validateRpcUrls(urls)
	} catch (err) {
		return { ok: false, results: [], error: err instanceof Error ? err.message : String(err) }
	}

	const results = await Promise.all(
		urls.map(async (url) => {
			try {
				const chainId = await withTimeout(deps.fetchChainId(url), PROBE_TIMEOUT_MS, "RPC check")
				if (expectedChainId !== undefined && chainId !== expectedChainId) {
					return { url, chainId, error: `RPC reports chain ${chainId}, expected ${expectedChainId}` }
				}
				return { url, chainId }
			} catch (err) {
				return { url, error: err instanceof Error ? err.message : String(err) }
			}
		}),
	)
	return { ok: results.every((r) => !r.error), results }
}

/** Warning-only: bundler probes never block the wizard. */
async function validateBundler(body: Record<string, unknown>, deps: Required<SetupDeps>) {
	const url = String(body.url ?? "").trim()
	if (!url) return { ok: false, warning: "Bundler URL is empty" }
	try {
		const entryPoints = await withTimeout(
			deps.rpcRequest(url, "eth_supportedEntryPoints", []),
			PROBE_TIMEOUT_MS,
			"Bundler probe",
		)
		return { ok: true, entryPoints }
	} catch (err) {
		return { ok: true, warning: `Bundler did not answer eth_supportedEntryPoints: ${err instanceof Error ? err.message : err}` }
	}
}

async function validateToken(body: Record<string, unknown>) {
	const rpcUrl = String(body.rpcUrl ?? "").trim()
	const address = String(body.address ?? "").trim()
	if (!isAddress(address)) return { ok: false, error: "Invalid token address" }
	if (!rpcUrl) return { ok: false, error: "Provide the chain's RPC URL first" }

	// No config exists yet, so read via a throwaway viem client instead of ChainClientManager.
	const client = createPublicClient({ transport: http(rpcUrl) })
	const code = await withTimeout(client.getCode({ address }), PROBE_TIMEOUT_MS, "Token bytecode check")
	if (!code || code === "0x") return { ok: false, error: "No contract deployed at this address" }

	const [symbol, decimals] = await withTimeout(
		Promise.all([
			client.readContract({ address, abi: ERC20_ABI, functionName: "symbol", args: [] }),
			client.readContract({ address, abi: ERC20_ABI, functionName: "decimals", args: [] }),
		]),
		PROBE_TIMEOUT_MS,
		"Token metadata read",
	)
	return { ok: true, symbol: symbol as string, decimals: Number(decimals) }
}

function deriveEvmAddress(body: Record<string, unknown>, res: ServerResponse): void {
	const privateKey = String(body.privateKey ?? "").trim()
	if (!/^0x[0-9a-fA-F]{64}$/.test(privateKey)) {
		return sendJson(res, 400, { error: "Expected 0x followed by 64 hex characters" })
	}
	return sendJson(res, 200, { address: privateKeyToAccount(privateKey as `0x${string}`).address })
}

async function substrateKey(body: Record<string, unknown>) {
	const key = body.key === undefined ? undefined : String(body.key)
	if (key) {
		const pair = await deriveSubstrateKeyPair(key)
		return { address: pair.address }
	}
	return generateSubstrateKey()
}

async function checkSubstrateBalance(body: Record<string, unknown>) {
	const wsUrl = String(body.wsUrl ?? "").trim()
	const key = String(body.key ?? "").trim()
	if (!wsUrl) throw new Error("wsUrl is required")
	if (!key) throw new Error("key is required")

	const pair = await deriveSubstrateKeyPair(key)
	const { ApiPromise, WsProvider } = await import("@polkadot/api")
	const provider = new WsProvider(wsUrl, 1_000)
	const api = await withTimeout(ApiPromise.create({ provider, throwOnConnect: true }), 20_000, "Hyperbridge connection")
	try {
		// biome-ignore lint/suspicious/noExplicitAny: polkadot API type
		const account = (await api.query.system.account(pair.address)) as any
		const decimals = (api.registry.chainDecimals as number[])[0] ?? 12
		const free = BigInt(account.data.free.toString())
		const reserved = BigInt(account.data.reserved.toString())
		return {
			address: pair.address,
			free: free.toString(),
			reserved: reserved.toString(),
			decimals,
			funded: free > 0n,
		}
	} finally {
		await api.disconnect().catch(() => {})
	}
}

interface GatedConfig {
	config: FillerTomlConfig
	toml: string
	chainLabels?: string[]
}

/** The same gate the CLI wizard applies before writing: reject anything `run` would reject. */
function gateConfig(body: Record<string, unknown>): GatedConfig | { ok: false; error: string } {
	const config = body.config as FillerTomlConfig | undefined
	const chainLabels = Array.isArray(body.chainLabels) ? body.chainLabels.map(String) : undefined
	if (!config || typeof config !== "object") return { ok: false, error: "Missing config object" }
	try {
		validateSignerConfig(config.simplex?.signer as SignerConfig)
		for (const chain of config.chains ?? []) validateRpcUrls(chain.rpcUrls)
		validateConfig(config)
		const toml = emitFillerToml(config, { chainComments: chainLabels })
		return { config, toml, chainLabels }
	} catch (err) {
		return { ok: false, error: err instanceof Error ? err.message : String(err) }
	}
}

/** Display-only TOML with every secret masked; the round-trip gate runs on the real config. */
export function maskToml(config: FillerTomlConfig, chainLabels?: string[]): string {
	const masked: FillerTomlConfig = JSON.parse(JSON.stringify(config))
	const signer = masked.simplex.signer as Record<string, string> | undefined
	if (signer) {
		for (const field of ["key", "apiToken", "apiPrivateKey"]) {
			if (signer[field]) signer[field] = maskSecret(signer[field])
		}
	}
	if (masked.simplex.substratePrivateKey) {
		masked.simplex.substratePrivateKey = maskSecret(masked.simplex.substratePrivateKey)
	}
	if (masked.binance) {
		masked.binance.apiKey = maskSecret(masked.binance.apiKey)
		masked.binance.apiSecret = maskSecret(masked.binance.apiSecret)
	}
	for (const chain of masked.chains) {
		chain.rpcUrls = chain.rpcUrls.map(maskUrlKey)
		chain.bundlerUrl = maskUrlKey(chain.bundlerUrl)
	}
	return emitFillerToml(masked, { chainComments: chainLabels })
}

/** Masks provider API keys embedded in URL paths/queries (Alchemy, Pimlico, …). */
function maskUrlKey(url: string): string {
	try {
		const parsed = new URL(url)
		if (parsed.searchParams.has("apikey")) {
			parsed.searchParams.set("apikey", maskSecret(parsed.searchParams.get("apikey")!))
			return parsed.toString()
		}
		const segments = parsed.pathname.split("/")
		const last = segments[segments.length - 1]
		if (last && last.length >= 16) {
			segments[segments.length - 1] = maskSecret(last)
			parsed.pathname = segments.join("/")
			return parsed.toString()
		}
		return url
	} catch {
		return url
	}
}

function saveAndStart(server: UiServer, setup: SetupContext, body: Record<string, unknown>, res: ServerResponse): void {
	if (server.getStartState() === "starting") {
		return sendJson(res, 409, { error: "A start is already in progress" })
	}
	const result = gateConfig(body)
	if ("error" in result) return sendJson(res, 400, result)

	const path = typeof body.path === "string" && body.path.trim() ? body.path.trim() : setup.configPath
	try {
		writeFileSync(path, result.toml, { mode: 0o600 })
		chmodSync(path, 0o600)
	} catch (err) {
		return sendJson(res, 500, { error: `Could not write ${path}: ${err instanceof Error ? err.message : err}` })
	}

	server.setStartState("starting")
	sendJson(res, 202, { ok: true, configPath: path, status: "starting" })

	// Booting takes tens of seconds (chain resolution, venue hydration, delegation);
	// the browser polls /api/setup/start-status. The onSaveAndStart callback flips
	// the server into operator mode on success.
	void setup.onSaveAndStart(result.config, result.toml, path).catch((err) => {
		const message = err instanceof Error ? err.message : String(err)
		logger.error({ err }, "Filler failed to start from the wizard config")
		server.setStartState("failed", message)
	})
}
