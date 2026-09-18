import { DEFAULT_ORDERBOOK_URL } from "@/config/defaults"
import { ChainConfigService, type HexString } from "@hyperbridge/sdk"
import { AssetRegistry, normalizeSymbol, registrySymbols, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import type { PairConfig } from "@/config/pairs"
import type { ChainConfirmationPolicy, FillerTomlConfig } from "@/config/filler-toml"
import { normalizeConfirmationPolicyKeys } from "./confirmation-keys"

/** The pre-pair-engine strategy rows, as far as migration needs them. */
interface LegacyStrategy {
	type: "stable" | "hyperfx"
	bpsCurve?: Array<{ amount: string; value: number }>
	maxOrderUsd?: number
	spreadBps?: number
	token1?: Record<string, HexString>
	bidPriceCurve?: Array<{ amount: string; price: string }>
	askPriceCurve?: Array<{ amount: string; price: string }>
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
	vault?: {
		uniswapV4?: {
			positions?: Array<{ chain: string; tokenId: string; referencePrice?: string; maxDeviationBps?: number }>
			side?: "bid" | "ask"
		}
	}
}

/**
 * Rewrites a pre-pair-engine config in place: `[[strategies]]` becomes
 * `[[pairs]]` (stable → same-token markets with bps margins mapped to
 * below-par ask prices; hyperfx → a USDC/<symbol> pair), per-strategy
 * confirmation policies move top-level. Returns human-readable notes about what
 * moved, including anything the current config no longer supports.
 */
export function migrateLegacyConfig(config: FillerTomlConfig): string[] {
	const legacy = config as FillerTomlConfig & { strategies?: LegacyStrategy[] }
	const strategies = legacy.strategies
	if (!strategies?.length) {
		delete legacy.strategies
		return []
	}

	const notes: string[] = []
	const pairs: PairConfig[] = [...(config.pairs ?? [])]
	// Keys are normalized on both sides so top-level entries genuinely win over
	// per-strategy ones — "EVM-8453" and "8453" are the same chain.
	const confirmationPolicies = normalizeConfirmationPolicyKeys(config.confirmationPolicies ?? {})

	for (const strategy of strategies) {
		if (strategy.type === "stable" && strategy.bpsCurve?.length) {
			let clamped = false
			const askPriceCurve = strategy.bpsCurve.map((point) => {
				const price = (10_000 - point.value) / 10_000
				const bounded = Math.min(0.9999, Math.max(0.0001, price))
				if (bounded !== price) clamped = true
				return { amount: String(point.amount), price: String(bounded) }
			})
			const maxOrderSize = strategy.maxOrderUsd !== undefined ? String(strategy.maxOrderUsd) : "100000"
			for (const symbol of ["USDC", "USDT"]) {
				if (pairs.some((p) => normalizeSymbol(p.token0) === symbol && normalizeSymbol(p.token1) === symbol)) continue
				pairs.push({
					token0: symbol,
					token1: symbol,
					maxOrderSize,
					askPriceCurve: askPriceCurve.map((point) => ({ ...point })),
				})
			}
			notes.push(
				`stable strategy became the USDC/USDC and USDT/USDT transfer pairs (bps margins mapped to below-par ask prices, order cap ${maxOrderSize}${strategy.maxOrderUsd !== undefined ? " from the legacy maxOrderUsd" : " by default"})`,
			)
			if (clamped) {
				notes.push(
					"some bps margins mapped outside the valid (0, 1) ask-price range and were clamped to [0.0001, 0.9999] — review the curves",
				)
			}
		}

		if (strategy.type === "hyperfx" && strategy.token1) {
			let token1 = resolveRegistrySymbol(strategy.token1)
			if (!token1) {
				token1 = nextFreeTokenName(config.assets)
				config.assets = { ...(config.assets ?? {}), [token1]: strategy.token1 }
				notes.push(
					`hyperfx exotic token did not match a registry symbol — kept as [assets.${token1}]; rename the symbol to taste`,
				)
			}
			const alreadyDeclared = pairs.some(
				(p) =>
					(normalizeSymbol(p.token0) === "USDC" && normalizeSymbol(p.token1) === normalizeSymbol(token1)) ||
					(normalizeSymbol(p.token0) === normalizeSymbol(token1) && normalizeSymbol(p.token1) === "USDC"),
			)
			if (alreadyDeclared) {
				notes.push(`hyperfx strategy for ${token1} skipped — a USDC/${token1} pair is already declared`)
			} else {
				pairs.push({
					token0: "USDC",
					token1,
					maxOrderSize: String(strategy.maxOrderUsd ?? 5000),
					...(strategy.bidPriceCurve?.length ? { bidPriceCurve: strategy.bidPriceCurve } : {}),
					...(strategy.askPriceCurve?.length ? { askPriceCurve: strategy.askPriceCurve } : {}),
				})
				notes.push(`hyperfx strategy became the USDC/${token1} pair`)
			}

			if (strategy.vault?.uniswapV4) {
				notes.push("dropped [strategies.vault.uniswapV4] — Uniswap V4 funding is no longer supported")
			}
		}

		for (const [chainId, policy] of Object.entries(
			normalizeConfirmationPolicyKeys(strategy.confirmationPolicies ?? {}),
		)) {
			confirmationPolicies[chainId] ??= policy
		}
	}

	delete legacy.strategies
	config.pairs = pairs
	// A legacy config predates the orderbook, and simplex has no prices without one.
	if (!config.orderbook) {
		config.orderbook = { url: DEFAULT_ORDERBOOK_URL }
		notes.push(`Added [orderbook] pointing at ${DEFAULT_ORDERBOOK_URL}; simplex prices fills from limit orders there.`)
	}
	if (Object.keys(confirmationPolicies).length > 0) {
		config.confirmationPolicies = confirmationPolicies
	}
	return notes
}

/** First TOKEN<n> name not already taken in the `[assets]` table. */
function nextFreeTokenName(assets: FillerTomlConfig["assets"]): string {
	for (let index = 1; ; index++) {
		const name = `TOKEN${index}`
		if (!Object.keys(assets ?? {}).some((key) => normalizeSymbol(key) === name)) return name
	}
}

/**
 * Matches a legacy per-chain address map against the shipped registry: a
 * symbol wins when at least one configured address is that symbol's deployment
 * and no chain contradicts it. Chains the registry has no address for carry
 * no signal either way — they must not veto an otherwise clean match.
 */
function resolveRegistrySymbol(token1: Record<string, HexString>): string | undefined {
	const registry = new AssetRegistry(new ChainConfigService({}))
	const entries = Object.entries(token1)
	if (entries.length === 0) return undefined
	for (const symbol of registrySymbols()) {
		if (USD_STABLE_SYMBOLS.has(symbol)) continue
		let matched = false
		let contradicted = false
		for (const [chain, address] of entries) {
			const known = registry.getAddress(symbol, chain)
			if (!known) continue
			if (known.toLowerCase() === address.toLowerCase()) matched = true
			else contradicted = true
		}
		if (matched && !contradicted) return symbol
	}
	return undefined
}
