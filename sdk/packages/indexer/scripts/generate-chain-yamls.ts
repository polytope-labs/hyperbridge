#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

import Handlebars from "handlebars"
import { RpcWebSocketClient } from "rpc-websocket-client"
import { Hex, hexToNumber } from "viem"

import { type Configuration, getConfigs, getEnv, getValidChains, loadConfig } from "../src/configs"
import { POOL_TOKENS as PREVIOUS_POOL_TOKENS } from "../src/addresses/pool-tokens.generated"

const skipRpc = process.argv.includes("--skip-rpc")
const root = process.cwd()
const currentEnv = getEnv()
const validChains = skipRpc
	? new Map<string, Configuration>(Object.entries(getConfigs()))
	: getValidChains()

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Load and compile templates
const templatesDir = path.join(__dirname, "templates")
const partialsDir = path.join(templatesDir, "partials")

// Register partials
Handlebars.registerPartial("handlers", fs.readFileSync(path.join(partialsDir, "handlers.hbs"), "utf8"))
Handlebars.registerPartial("metadata", fs.readFileSync(path.join(partialsDir, "metadata.hbs"), "utf8"))
Handlebars.registerPartial("network-config", fs.readFileSync(path.join(partialsDir, "network-config.hbs"), "utf8"))

// Compile templates
const substrateTemplate = Handlebars.compile(
	fs.readFileSync(path.join(templatesDir, "substrate-chain.yaml.hbs"), "utf8"),
)
const evmTemplate = Handlebars.compile(fs.readFileSync(path.join(templatesDir, "evm-chain.yaml.hbs"), "utf8"))
const multichainTemplate = Handlebars.compile(fs.readFileSync(path.join(templatesDir, "multichain.yaml.hbs"), "utf8"))

const EVM_TRACKED = [
	// Envrionment Variable Tracked
	"COIN_GECKGO_API_KEY",
] as const

const getChainTypesPath = (chain: string) => {
	// Extract base chain name before the hyphen
	const baseChainName = chain.split("-")[0]
	const potentialPath = `./dist/substrate-chaintypes/${baseChainName}.js`

	// Check if file exists
	if (fs.existsSync(potentialPath)) {
		return potentialPath
	}
	return null
}

const generateEndpoints = (chain: string) => {
	const envKey = chain.replace(/-/g, "_").toUpperCase()
	// Expect comma-separated endpoints in env var
	return process.env[envKey]?.split(",") || []
}

// Generate chain-specific YAML files
const generateSubstrateYaml = async (chain: string, config: Configuration) => {
	const chainTypesConfig = getChainTypesPath(chain)
	const endpoints = generateEndpoints(chain)

	let blockNumber: number
	// Only connect to RPC when we actually need the live head (local/nexus-ci).
	// For other environments we use the static startBlock from config.
	if (skipRpc || (currentEnv !== "local" && currentEnv !== "nexus-ci")) {
		blockNumber = config.startBlock
	} else {
		// Expect comma-separated endpoints in env var
		const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
		const rpc = new RpcWebSocketClient()
		await rpc.connect(rpcUrl as string)
		const header = (await rpc.call("chain_getHeader", [])) as { number: Hex }
		blockNumber = hexToNumber(header.number)
	}

	// Check if this is a Hyperbridge chain (stateMachineId is KUSAMA-4009 or POLKADOT-3367)
	const isHyperbridgeChain = ["KUSAMA-4009", "POLKADOT-3367"].includes(config.stateMachineId)

	// Check if price indexing should be enabled (Hyperbridge chain but not testnet)
	const enablePriceIndexing = isHyperbridgeChain && currentEnv !== "testnet"

	const templateData = {
		name: `${chain}-chain`,
		description: `${chain.charAt(0).toUpperCase() + chain.slice(1)} Chain Indexer`,
		runner: {
			node: {
				name: "@subql/node",
				version: ">=4.0.0",
			},
		},
		config,
		endpoints,
		chainTypesConfig,
		blockNumber,
		isHyperbridgeChain,
		enablePriceIndexing,
		handlerKind: "substrate/EventHandler",
		handlers: [
			{ handler: "handleIsmpStateMachineUpdatedEvent", module: "ismp", method: "StateMachineUpdated" },
			{ handler: "handleSubstrateRequestEvent", module: "ismp", method: "Request" },
			{ handler: "handleSubstrateResponseEvent", module: "ismp", method: "Response" },
			{ handler: "handleSubstratePostRequestHandledEvent", module: "ismp", method: "PostRequestHandled" },
			{
				handler: "handleSubstratePostRequestTimeoutHandledEvent",
				module: "ismp",
				method: "PostRequestTimeoutHandled",
			},
			{ handler: "handleSubstrateGetRequestHandledEvent", module: "ismp", method: "GetRequestHandled" },
			{
				handler: "handleSubstrateGetRequestTimeoutHandledEvent",
				module: "ismp",
				method: "GetRequestTimeoutHandled",
			},
		],
	}

	return substrateTemplate(templateData)
}

const generateEvmYaml = async (chain: string, config: Configuration) => {
	const endpoints = generateEndpoints(chain)

	let blockNumber: number
	// Only connect to RPC when we actually need the live head (local env).
	// For other environments we use the static startBlock from config.
	if (skipRpc || currentEnv !== "local") {
		blockNumber = config.startBlock
	} else {
		// Expect comma-separated endpoints in env var
		const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
		const response = await fetch(rpcUrl as string, {
			method: "POST",
			headers: {
				accept: "application/json",
				"content-type": "application/json",
			},
			body: JSON.stringify({
				id: 1,
				jsonrpc: "2.0",
				method: "eth_blockNumber",
			}),
		})
		const data = await response.json()
		blockNumber = hexToNumber(data.result)
	}

	const templateData = {
		name: chain,
		description: `${chain.charAt(0).toUpperCase() + chain.slice(1)} Indexer`,
		runner: {
			node: {
				name: "@subql/node-ethereum",
				version: ">=3.0.0",
			},
		},
		config,
		endpoints,
		blockNumber,
		// Flattened (vault, underlyingToken) pairs so the template can emit one Deposit/Withdraw
		// datasource per vault. The handler resolves underlyingToken from YIELD_VAULT_ADDRESSES.
		yieldVaults:
			config.type === "evm" && config.contracts?.yieldVaults
				? Object.entries(config.contracts.yieldVaults).flatMap(([token, entry]) =>
						entry.vaults.map((vault) => ({ vault, underlyingToken: token })),
					)
				: [],
		handlerKind: "ethereum/LogHandler",
		handlers: [
			{ handler: "handleStateMachineUpdatedEvent", topics: ["StateMachineUpdated(string,uint256)"] },
			{
				handler: "handlePostRequestEvent",
				topics: ["PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)"],
			},
			{ handler: "handlePostRequestHandledEvent", topics: ["PostRequestHandled(bytes32,address)"] },
			{ handler: "handlePostRequestTimeoutHandledEvent", topics: ["PostRequestTimeoutHandled(bytes32,string)"] },
			{
				handler: "handleGetRequestEvent",
				topics: ["GetRequestEvent(string,string,bytes,bytes[],uint256,uint256,uint256,bytes,uint256)"],
			},
			{ handler: "handleGetRequestHandledEvent", topics: ["GetRequestHandled(bytes32,address)"] },
			{ handler: "handleGetRequestTimeoutHandledEvent", topics: ["GetRequestTimeoutHandled(bytes32,string)"] },
		],
	}

	return evmTemplate(templateData)
}

async function generateAllChainYamls() {
	for (const [chain, config] of validChains) {
		const yaml =
			config.type === "substrate"
				? await generateSubstrateYaml(chain, config)
				: await generateEvmYaml(chain, config)

		fs.writeFileSync(root + `/src/configs/${chain}.yaml`, yaml)
		console.log(`Generated ${root}/src/configs/${chain}.yaml`)
	}
}

const generateMultichainYaml = () => {
	const projects = Array.from(validChains.keys()).map((chain) => `./${chain}.yaml`)

	const templateData = {
		projects,
	}

	const yaml = multichainTemplate(templateData)
	fs.writeFileSync(root + "/src/configs/subquery-multichain.yaml", yaml)
	console.log("Generated subquery-multichain.yaml")
}

const generateChainIdsByGenesis = () => {
	const chainIdsByGenesis = {}

	validChains.forEach((config) => {
		if (config.chainId) {
			chainIdsByGenesis[config.chainId] = config.stateMachineId
		}
	})

	const chainIdsByGenesisContent = `// Auto-generated, DO NOT EDIT \nexport const CHAIN_IDS_BY_GENESIS = ${JSON.stringify(chainIdsByGenesis, null, 2)}`

	fs.writeFileSync(root + "/src/chain-ids-by-genesis.ts", chainIdsByGenesisContent)
	console.log("Generated chain-ids-by-genesis.ts")
}

const generateChainsByIsmpHost = () => {
	const chainsByIsmpHost = {}

	validChains.forEach((config) => {
		// Only include EVM chains with ethereumHost contract
		if (config.type === "evm" && config.contracts?.ethereumHost) {
			chainsByIsmpHost[config.stateMachineId] = config.contracts.ethereumHost
		}
	})

	const chainsByIsmpHostContent = `// Auto-generated, DO NOT EDIT \nexport const CHAINS_BY_ISMP_HOST = ${JSON.stringify(chainsByIsmpHost, null, 2)}`

	fs.writeFileSync(root + "/src/chains-by-ismp-host.ts", chainsByIsmpHostContent)
	console.log("Generated chains-by-ismp-host.ts")
}

const generateChainsIntentGatewayV3Addresses = () => {
	const intentGatewayV3 = {}

	validChains.forEach((config) => {
		if (config.type === "evm" && config.contracts?.intentGatewayV3) {
			intentGatewayV3[config.stateMachineId] = config.contracts.intentGatewayV3
		}
	})

	const value = `// Auto-generated, DO NOT EDIT \nexport const INTENT_GATEWAY_V3_ADDRESSES = ${JSON.stringify(intentGatewayV3, null, 2)}`

	fs.writeFileSync(root + "/src/intent-gateway-v3-addresses.ts", value)
	console.log("Generated intent-gateway-v3-addresses.ts")
}

const generateYieldVaultAddresses = () => {
	const lines: string[] = []
	lines.push("// Auto-generated, DO NOT EDIT")
	lines.push("// ERC-4626 vault addresses per chain, keyed by underlying token address (lowercase).")
	lines.push("// Aave stata token addresses sourced from https://github.com/bgd-labs/aave-address-book")
	lines.push("// Values are arrays because multiple vaults may wrap the same underlying token.")
	lines.push(
		"// To add or update vaults, edit the \"yieldVaults\" field in the relevant chain entry",
	)
	lines.push("// in src/configs/config-mainnet.json (or config-testnet.json) and re-run codegen.")
	lines.push("export const YIELD_VAULT_ADDRESSES: Record<string, Record<string, string[]>> = {")

	let firstChain = true
	validChains.forEach((config) => {
		if (config.type !== "evm" || !config.contracts?.yieldVaults) return
		if (!firstChain) lines.push("")
		firstChain = false
		lines.push(`\t// ${config.stateMachineId}`)
		lines.push(`\t"${config.stateMachineId}": {`)
		const entries = Object.entries(config.contracts.yieldVaults)
		entries.forEach(([token, entry], i) => {
			lines.push(`\t\t// ${entry.description}`)
			const vaultsJson = JSON.stringify(entry.vaults)
			const comma = i < entries.length - 1 ? "," : ""
			lines.push(`\t\t"${token}": ${vaultsJson}${comma}`)
		})
		lines.push("\t},")
	})

	lines.push("}")
	lines.push("")

	fs.writeFileSync(root + "/src/yield-vault-addresses.ts", lines.join("\n"))
	console.log("Generated yield-vault-addresses.ts")
}

// The pool-token registry is the yieldVaults key set — the same addresses, resolved to the
// (symbol, decimals) the pool layer needs. Both are read off the same list on purpose: a token
// the sweep tracks and a token that maps to a pool must not be able to drift apart.
//
// Symbols and decimals are read from the token contracts rather than written by hand, because
// both are silent when wrong — a bad decimal scales every published rate for the pair by a
// power of ten without erroring anywhere.
const ERC20_SYMBOL = "0x95d89b41"
const ERC20_DECIMALS = "0x313ce567"

// On-chain `symbol()` is the token's own branding, not a pool identity: the symbol is what
// collapses two legs on different chains into one pool, so it has to be canonicalised before
// use. Left unmapped, polygon's "USDT0", arbitrum's "USD₮0" and Asset Hub's "USDt" would each
// become a pool separate from ethereum's "USDT".
//
// Keyed by the lowercased on-chain symbol, so this table does two jobs at once: it fixes casing
// ("usdt" -> "USDT", "cngn" -> "cNGN") and maps genuine aliases onto the canonical asset. A
// token whose symbol is absent here keeps its on-chain value verbatim.
const SYMBOL_ALIASES: Record<string, string> = {
	// canonical casing
	usdc: "USDC",
	usdt: "USDT", // also catches Asset Hub's "USDt"
	dai: "DAI",
	cngn: "cNGN",
	zarp: "ZARP",
	eurc: "EURC",
	xsgd: "XSGD",
	tryb: "TRYB",
	usdr: "USDR",
	// aliases for the same asset under another name
	usdt0: "USDT", // polygon, and arbitrum's "USD₮0" once non-ASCII is stripped
	"usd₮0": "USDT",
	wxdai: "DAI", // gnosis: wrapped xDai is the DAI of that chain
	"usdc.e": "USDC", // bridged variants
	"usdt.e": "USDT",
	"dai.e": "DAI",
	// bsc-chapel's stand-in for USDC is "Hyper USD" (symbol "USD.h"). Mapped so testnet pool
	// slugs keep matching mainnet's; drop this line if testnet should read USD.h instead.
	"usd.h": "USDC",
}

const decodeAbiString = (hex: string): string | null => {
	if (!hex || hex === "0x") return null
	const body = hex.slice(2)
	// dynamic string: offset, length, data — anything shorter is a bytes32 symbol
	if (body.length >= 128) {
		const length = parseInt(body.slice(64, 128), 16)
		return Buffer.from(body.slice(128, 128 + length * 2), "hex").toString("utf8").trim()
	}
	return Buffer.from(body, "hex").toString("utf8").replace(/\0+/g, "").trim()
}

const ethCall = async (endpoints: string[], to: string, data: string): Promise<string | null> => {
	// Public endpoints throttle, and a throttled response is indistinguishable from "no such
	// token" if you only ask once. Rotate and retry before believing a null.
	for (let attempt = 0; attempt < 3; attempt++) {
		for (const url of endpoints) {
			try {
				const response = await fetch(url, {
					method: "POST",
					headers: { accept: "application/json", "content-type": "application/json" },
					body: JSON.stringify({
						id: 1,
						jsonrpc: "2.0",
						method: "eth_call",
						params: [{ to, data }, "latest"],
					}),
				})
				const body = await response.json()
				if (body.result && body.result !== "0x") return body.result as string
			} catch {
				// try the next endpoint
			}
		}
		await new Promise((resolve) => setTimeout(resolve, 400 * (attempt + 1)))
	}
	return null
}

const generatePoolTokens = async () => {
	// Env-independent: the registry is imported by handlers that run against either network, so
	// it carries every chain from both configs rather than only the one being built.
	const chains = new Map<string, Configuration>([
		...Object.entries(loadConfig("mainnet")),
		...Object.entries(loadConfig("testnet")),
	])

	const resolved: Record<string, Record<string, { symbol: string; decimals: number }>> = {}
	const notes: string[] = []

	for (const [chain, config] of chains) {
		if (config.type !== "evm" || !config.contracts?.yieldVaults) continue
		const stateMachineId = config.stateMachineId
		const previous = (PREVIOUS_POOL_TOKENS as Record<string, Record<string, { symbol: string; decimals: number }>>)[
			stateMachineId
		]
		const endpoints = generateEndpoints(chain)
		const tokens: Record<string, { symbol: string; decimals: number }> = {}

		for (const address of Object.keys(config.contracts.yieldVaults)) {
			const token = address.toLowerCase()
			const cached = previous?.[token]

			if (skipRpc || endpoints.length === 0) {
				if (cached) tokens[token] = cached
				else notes.push(`${stateMachineId} ${token}: no endpoint and no cached entry — omitted`)
				continue
			}

			const [symbolHex, decimalsHex] = [
				await ethCall(endpoints, token, ERC20_SYMBOL),
				await ethCall(endpoints, token, ERC20_DECIMALS),
			]
			const decimals = decimalsHex ? parseInt(decimalsHex, 16) : null
			const rawSymbol = decodeAbiString(symbolHex ?? "")

			if (decimals === null || !Number.isFinite(decimals)) {
				// Never guess decimals. Keep the last known-good value and say so loudly.
				if (cached) {
					tokens[token] = cached
					notes.push(`${stateMachineId} ${token}: decimals() unreadable — kept cached ${cached.decimals}`)
				} else {
					notes.push(`${stateMachineId} ${token}: decimals() unreadable and no cached entry — omitted`)
				}
				continue
			}

			const ascii = (rawSymbol ?? "").replace(/[^\x20-\x7e]/g, "")
			const symbol =
				SYMBOL_ALIASES[(rawSymbol ?? "").toLowerCase()] ??
				SYMBOL_ALIASES[ascii.toLowerCase()] ??
				(ascii || cached?.symbol)

			if (!symbol) {
				notes.push(`${stateMachineId} ${token}: symbol() unreadable and no cached entry — omitted`)
				continue
			}
			if (cached && cached.decimals !== decimals) {
				notes.push(`${stateMachineId} ${token}: decimals changed ${cached.decimals} -> ${decimals}`)
			}
			tokens[token] = { symbol, decimals }
		}

		if (Object.keys(tokens).length > 0) resolved[stateMachineId] = tokens
	}

	const lines: string[] = []
	lines.push("// Auto-generated, DO NOT EDIT")
	lines.push("// The tokens that liquidity pools are composed from, keyed by state machine id then")
	lines.push("// lowercase token address. Addresses are the \"yieldVaults\" keys of the relevant chain")
	lines.push("// entry in src/configs/config-mainnet.json / config-testnet.json; symbols and decimals")
	lines.push("// are read from each token contract at generation time.")
	lines.push("//")
	lines.push("// Decimals are per chain because the same symbol is not the same everywhere (BSC stables")
	lines.push("// are 18-decimal, most others 6); every rate and depth normalization depends on them.")
	lines.push("// To add a token, add it to that chain's \"yieldVaults\" and re-run codegen with an RPC")
	lines.push("// endpoint configured for the chain. Committed rather than gitignored: it is derived")
	lines.push("// from chain state, not from anything else in the repo, so --skip-rpc builds reuse it.")
	lines.push("export const POOL_TOKENS: Record<string, Record<string, { symbol: string; decimals: number }>> = {")
	const chainIds = Object.keys(resolved)
	chainIds.forEach((stateMachineId, chainIndex) => {
		lines.push(`\t"${stateMachineId}": {`)
		const entries = Object.entries(resolved[stateMachineId])
		entries.forEach(([token, { symbol, decimals }], i) => {
			const comma = i < entries.length - 1 ? "," : ""
			lines.push(`\t\t"${token}": { symbol: "${symbol}", decimals: ${decimals} }${comma}`)
		})
		lines.push(chainIndex < chainIds.length - 1 ? "\t}," : "\t},")
	})
	lines.push("}")
	lines.push("")

	fs.writeFileSync(root + "/src/addresses/pool-tokens.generated.ts", lines.join("\n"))
	const total = Object.values(resolved).reduce((n, t) => n + Object.keys(t).length, 0)
	console.log(`Generated pool-tokens.generated.ts (${total} tokens across ${chainIds.length} chains)`)
	for (const note of notes) console.warn(`  pool-tokens: ${note}`)
}

const generateSolverAccountAddresses = () => {
	const solverAccounts: Record<string, string> = {}

	validChains.forEach((config) => {
		if (config.type === "evm" && config.contracts?.solverAccount) {
			solverAccounts[config.stateMachineId] = config.contracts.solverAccount
		}
	})

	const value = `// Auto-generated, DO NOT EDIT
// SolverAccount contract address per chain (EIP-7702 delegation target for our solver EOAs).
// To add or update entries, edit the "solverAccount" field in the relevant chain entry
// in src/configs/config-mainnet.json (or config-testnet.json) and re-run codegen.
export const SOLVER_ACCOUNT_ADDRESSES: Record<string, string> = ${JSON.stringify(solverAccounts, null, 2)}`

	fs.writeFileSync(root + "/src/solver-account-addresses.ts", value)
	console.log("Generated solver-account-addresses.ts")
}

const generateTokenSlotOverrides = () => {
	const lines: string[] = []
	lines.push("// Auto-generated, DO NOT EDIT")
	lines.push("// Per-token ERC-20 storage slot overrides for tokens that don't follow the standard OZ layout")
	lines.push("// (slot 0 = _balances, slot 1 = _allowances).")
	lines.push("// To add or update entries, edit the \"tokenSlots\" field in the relevant chain entry")
	lines.push("// in src/configs/config-mainnet.json (or config-testnet.json) and re-run codegen.")
	lines.push(
		"export const TOKEN_SLOT_OVERRIDES: Record<string, { balanceSlot: bigint; allowanceSlot: bigint }> = {",
	)

	const seen = new Map<string, { description: string; balanceSlot: number; allowanceSlot: number }>()
	validChains.forEach((config) => {
		if (config.type !== "evm" || !config.contracts?.tokenSlots) return
		Object.entries(config.contracts.tokenSlots).forEach(([token, entry]) => {
			seen.set(token.toLowerCase(), entry)
		})
	})

	const entries = Array.from(seen.entries())
	entries.forEach(([token, entry], i) => {
		lines.push(`\t// ${entry.description}`)
		const comma = i < entries.length - 1 ? "," : ""
		lines.push(
			`\t"${token}": { balanceSlot: ${entry.balanceSlot}n, allowanceSlot: ${entry.allowanceSlot}n }${comma}`,
		)
	})

	lines.push("}")
	lines.push("")

	fs.writeFileSync(root + "/src/token-slot-overrides.ts", lines.join("\n"))
	console.log("Generated token-slot-overrides.ts")
}

const generateTestnetStateMachineIds = () => {
	const testnetStateMachineIds = new Set()

	// Only generate testnet state machine IDs when in testnet environment
	if (currentEnv === "testnet") {
		validChains.forEach((config) => {
			testnetStateMachineIds.add(config.stateMachineId)
		})
	}

	const value = `// Auto-generated, DO NOT EDIT \nexport const TESTNET_STATE_MACHINE_IDS: string[] = ${JSON.stringify(Array.from(testnetStateMachineIds), null, 2)}`

	fs.writeFileSync(root + "/src/testnet-state-machine-ids.ts", value)
	console.log("Generated testnet-state-machine-ids.ts")
}

const generateEnvironmentConfig = () => {
	const configurations = {}

	// Set evm and substrate environment configurations
	validChains.forEach((config, chain) => {
		const envKey = chain.replace(/-/g, "_").toUpperCase()
		const endpoints = process.env[envKey]?.split(",") || []

		if (endpoints.length > 0) {
			configurations[config.stateMachineId] = endpoints[0].trim()
		}
	})

	EVM_TRACKED.forEach((e: string) => (configurations[e] = process.env?.[e] ?? null))

	fs.writeFileSync(root + "/src/env-config.json", JSON.stringify(configurations, null, 2))
	console.log("Generated env-config.json")

	// If a built dist bundle exists, patch the inlined env-config so the running
	// indexer picks up the updated RPC endpoints and API keys without rebuilding.
	const distBundle = path.join(root, "dist", "index.js")
	if (fs.existsSync(distBundle)) {
		const bundle = fs.readFileSync(distBundle, "utf8")
		// The bundler inlines env-config.json as JSON.parse('{...}'). Anchor on
		// the COIN_GECKGO_API_KEY marker which is always present in the config.
		const envConfigPattern = /JSON\.parse\('(\{[^']*COIN_GECKGO_API_KEY[^']*\})'\)/
		if (!envConfigPattern.test(bundle)) {
			console.warn("Could not find inlined env-config in dist/index.js; skipping patch")
			return
		}
		// Escape any single quotes and backslashes so the replacement is a valid JS string literal
		const serialized = JSON.stringify(configurations).replace(/\\/g, "\\\\").replace(/'/g, "\\'")
		const patched = bundle.replace(envConfigPattern, `JSON.parse('${serialized}')`)
		fs.writeFileSync(distBundle, patched)
		console.log("Patched env-config in dist/index.js")
	}
}

try {
	await generateAllChainYamls()
	generateMultichainYaml()
	generateChainIdsByGenesis()
	generateChainsByIsmpHost()
	generateChainsIntentGatewayV3Addresses()
	generateYieldVaultAddresses()
	await generatePoolTokens()
	generateSolverAccountAddresses()
	generateTokenSlotOverrides()
	generateTestnetStateMachineIds()
	generateEnvironmentConfig()
	process.exit(0)
} catch (err) {
	console.error("Error generating YAMLs:", err)
	process.exit(1)
}
