import { ChainConfigService, type HexString } from "@hyperbridge/sdk"
import { AssetRegistry, normalizeSymbol, registrySymbols, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import type { PairConfig } from "@/config/pairs"
import type { ChainConfirmationPolicy, FillerTomlConfig } from "@/config/filler-toml"

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
 * confirmation policies move top-level, and `[strategies.vault.uniswapV4]`
 * moves to `[vault.uniswapV4]`. Returns human-readable notes about what moved.
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
	const confirmationPolicies: Record<string, ChainConfirmationPolicy> = { ...(config.confirmationPolicies ?? {}) }

	for (const strategy of strategies) {
		if (strategy.type === "stable" && strategy.bpsCurve?.length) {
			for (const symbol of ["USDC", "USDT"]) {
				if (pairs.some((p) => normalizeSymbol(p.token0) === symbol && normalizeSymbol(p.token1) === symbol)) continue
				pairs.push({
					token0: symbol,
					token1: symbol,
					maxOrderSize: "100000",
					askPriceCurve: strategy.bpsCurve.map((point) => ({
						amount: String(point.amount),
						price: String((10_000 - point.value) / 10_000),
					})),
				})
			}
			notes.push(
				"stable strategy became the USDC/USDC and USDT/USDT transfer pairs (bps margins mapped to below-par ask prices, order cap 100000)",
			)
		}

		if (strategy.type === "hyperfx" && strategy.token1) {
			let token1 = resolveRegistrySymbol(strategy.token1)
			if (!token1) {
				token1 = "TOKEN1"
				config.assets = { ...(config.assets ?? {}), [token1]: strategy.token1 }
				notes.push(
					"hyperfx exotic token did not match a registry symbol — kept as [assets.TOKEN1]; rename the symbol to taste",
				)
			}
			pairs.push({
				token0: "USDC",
				token1,
				maxOrderSize: String(strategy.maxOrderUsd ?? 5000),
				...(strategy.bidPriceCurve?.length ? { bidPriceCurve: strategy.bidPriceCurve } : {}),
				...(strategy.askPriceCurve?.length ? { askPriceCurve: strategy.askPriceCurve } : {}),
			})
			notes.push(`hyperfx strategy became the USDC/${token1} pair`)

			if (strategy.vault?.uniswapV4) {
				config.vault = {
					...(config.vault ?? {}),
					uniswapV4: {
						...strategy.vault.uniswapV4,
						...(strategy.spreadBps !== undefined ? { spreadBps: strategy.spreadBps } : {}),
					},
				}
				notes.push("[strategies.vault.uniswapV4] moved to the top-level [vault.uniswapV4]")
			}
		}

		for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies ?? {})) {
			confirmationPolicies[chainId] ??= policy
		}
	}

	delete legacy.strategies
	config.pairs = pairs
	if (Object.keys(confirmationPolicies).length > 0) {
		config.confirmationPolicies = confirmationPolicies
	}
	return notes
}

/**
 * Matches a legacy per-chain address map against the shipped registry: a
 * symbol wins only when every configured address is that symbol's deployment
 * on its chain.
 */
function resolveRegistrySymbol(token1: Record<string, HexString>): string | undefined {
	const registry = new AssetRegistry(new ChainConfigService({}))
	const entries = Object.entries(token1)
	if (entries.length === 0) return undefined
	for (const symbol of registrySymbols()) {
		if (USD_STABLE_SYMBOLS.has(symbol)) continue
		const allMatch = entries.every(
			([chain, address]) => registry.getAddress(symbol, chain)?.toLowerCase() === address.toLowerCase(),
		)
		if (allMatch) return symbol
	}
	return undefined
}
