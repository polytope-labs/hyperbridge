#!/usr/bin/env node
// Refreshes src/addresses/pool-tokens.generated.ts from chain state.
//
// Deliberately NOT part of `codegen:yamls`. That script runs inside the assembled release
// tarball, which ships only dist/, src/configs/ and scripts/ — no src/addresses/ — so anything
// on the deployment codegen path must not touch this registry. Refreshing it is a development
// task: it needs RPC endpoints, and its output is committed and reviewed, because a wrong
// decimal silently rescales every published rate for that token's pairs.
//
// Usage: ENV=mainnet <CHAIN_ENV_VARS...> pnpm codegen:pool-tokens
import fs from "node:fs"

import { type Configuration, loadConfig } from "../src/configs"

const root = process.cwd()

type PoolTokenMap = Record<string, Record<string, { symbol: string; decimals: number }>>

// Loaded dynamically, not by a top-level import: this script is copied into the release tarball
// along with the rest of scripts/, where src/addresses/ does not exist. A static import there
// fails at module resolution with an ERR_MODULE_NOT_FOUND that says nothing about why.
const loadPreviousPoolTokens = async (): Promise<PoolTokenMap> => {
	try {
		const module = await import("../src/addresses/pool-tokens.generated")
		return module.POOL_TOKENS as PoolTokenMap
	} catch {
		throw new Error(
			"src/addresses/pool-tokens.generated.ts not found. This script refreshes a committed " +
				"registry and only runs inside the repository, not in an assembled release package.",
		)
	}
}

// Addresses come from each chain's "yieldVaults" keys, so the set a pool maps to and the set
// the balance sweep tracks are the same list read twice and cannot drift apart.
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

const generateEndpoints = (chain: string) => process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",") || []

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
					body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "eth_call", params: [{ to, data }, "latest"] }),
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

	const previousAll = await loadPreviousPoolTokens()
	const resolved: Record<string, Record<string, { symbol: string; decimals: number }>> = {}
	const warnings: string[] = []
	const failures: string[] = []

	for (const [chain, config] of chains) {
		if (config.type !== "evm" || !config.contracts?.yieldVaults) continue
		const stateMachineId = config.stateMachineId
		const previous = previousAll[stateMachineId]
		const endpoints = generateEndpoints(chain)
		const tokens: Record<string, { symbol: string; decimals: number }> = {}

		for (const address of Object.keys(config.contracts.yieldVaults)) {
			const token = address.toLowerCase()
			const cached = previous?.[token]

			if (endpoints.length === 0) {
				// No endpoint for this chain: keep what we already knew, never invent it.
				if (cached) {
					tokens[token] = cached
					warnings.push(`${stateMachineId} ${token}: no endpoint configured — kept committed entry`)
				} else {
					failures.push(`${stateMachineId} ${token}: no endpoint configured and no committed entry`)
				}
				continue
			}

			const symbolHex = await ethCall(endpoints, token, ERC20_SYMBOL)
			const decimalsHex = await ethCall(endpoints, token, ERC20_DECIMALS)
			const decimals = decimalsHex ? parseInt(decimalsHex, 16) : null
			const rawSymbol = decodeAbiString(symbolHex ?? "")

			if (decimals === null || !Number.isFinite(decimals)) {
				// Never guess decimals — a wrong one is silent and rescales every rate.
				if (cached) {
					tokens[token] = cached
					warnings.push(`${stateMachineId} ${token}: decimals() unreadable — kept committed ${cached.decimals}`)
				} else {
					failures.push(`${stateMachineId} ${token}: decimals() unreadable and no committed entry`)
				}
				continue
			}

			const ascii = (rawSymbol ?? "").replace(/[^\x20-\x7e]/g, "")
			const symbol =
				SYMBOL_ALIASES[(rawSymbol ?? "").toLowerCase()] ??
				SYMBOL_ALIASES[ascii.toLowerCase()] ??
				(ascii || cached?.symbol)

			if (!symbol) {
				failures.push(`${stateMachineId} ${token}: symbol() unreadable and no committed entry`)
				continue
			}
			if (cached && cached.decimals !== decimals) {
				warnings.push(`${stateMachineId} ${token}: decimals changed ${cached.decimals} -> ${decimals}`)
			}
			if (cached && cached.symbol !== symbol) {
				warnings.push(`${stateMachineId} ${token}: symbol changed ${cached.symbol} -> ${symbol}`)
			}
			tokens[token] = { symbol, decimals }
		}

		if (Object.keys(tokens).length > 0) resolved[stateMachineId] = tokens
	}

	for (const warning of warnings) console.warn(`  warning: ${warning}`)

	// A partially-resolved registry is worse than none: the missing tokens stop mapping to a
	// pool and stop being swept, silently. Refuse to overwrite the committed file.
	if (failures.length > 0) {
		for (const failure of failures) console.error(`  unresolved: ${failure}`)
		throw new Error(`${failures.length} token(s) could not be resolved — pool-tokens.generated.ts left unchanged`)
	}

	// Same guard for whole chains: never drop one that the committed registry already had.
	const dropped = Object.keys(previousAll).filter((id) => !(id in resolved))
	if (dropped.length > 0) {
		throw new Error(`would drop chain(s) ${dropped.join(", ")} — pool-tokens.generated.ts left unchanged`)
	}

	const lines: string[] = []
	lines.push("// Auto-generated, DO NOT EDIT")
	lines.push("// The tokens that liquidity pools are composed from, keyed by state machine id then")
	lines.push('// lowercase token address. Addresses are the "yieldVaults" keys of the relevant chain')
	lines.push("// entry in src/configs/config-mainnet.json / config-testnet.json; symbols and decimals")
	lines.push("// are read from each token contract by scripts/generate-pool-tokens.ts.")
	lines.push("//")
	lines.push("// Decimals are per chain because the same symbol is not the same everywhere (BSC stables")
	lines.push("// are 18-decimal, most others 6); every rate and depth normalization depends on them.")
	lines.push('// To add a token, add it to that chain\'s "yieldVaults" and run `pnpm codegen:pool-tokens`')
	lines.push("// with an RPC endpoint configured for the chain. Committed rather than gitignored: it")
	lines.push("// derives from chain state, which nothing else in the repo can reconstruct.")
	lines.push("export const POOL_TOKENS: Record<string, Record<string, { symbol: string; decimals: number }>> = {")
	const chainIds = Object.keys(resolved)
	chainIds.forEach((stateMachineId) => {
		lines.push(`\t"${stateMachineId}": {`)
		const entries = Object.entries(resolved[stateMachineId])
		entries.forEach(([token, { symbol, decimals }], i) => {
			const comma = i < entries.length - 1 ? "," : ""
			lines.push(`\t\t"${token}": { symbol: "${symbol}", decimals: ${decimals} }${comma}`)
		})
		lines.push("\t},")
	})
	lines.push("}")
	lines.push("")

	fs.writeFileSync(root + "/src/addresses/pool-tokens.generated.ts", lines.join("\n"))
	const total = Object.values(resolved).reduce((n, t) => n + Object.keys(t).length, 0)
	console.log(`Generated pool-tokens.generated.ts (${total} tokens across ${chainIds.length} chains)`)
}

try {
	await generatePoolTokens()
	process.exit(0)
} catch (err) {
	console.error("Error generating pool tokens:", err instanceof Error ? err.message : err)
	process.exit(1)
}
