import { confirm, log, select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import type { ChainConfirmationPolicy } from "@/config/filler-toml"
import type { PairConfig } from "@/config/pairs"
import { isRegistrySymbol, normalizeSymbol, registrySymbols, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import { guard, why, askText, askAddress } from "../prompt-utils"
import { normalizeConfirmationPolicyKeys } from "../confirmation-keys"
import { WHY } from "../help-text"
import { TESTNET_CONFIRMATION_POINTS, type Prefill, type WizardState } from "../state"

/** Registry symbols with USD stables first — the built-in candidates for either side. */
const SYMBOL_OPTIONS = [
	...registrySymbols().filter((symbol) => USD_STABLE_SYMBOLS.has(symbol)),
	...registrySymbols().filter((symbol) => !USD_STABLE_SYMBOLS.has(symbol)),
]

/** Near-par two-sided defaults for stable-to-stable markets. */
const STABLE_SWAP_BID = [{ amount: "0", price: "1.001" }]
const STABLE_SWAP_ASK = [{ amount: "0", price: "0.999" }]

export async function stepStrategies(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.pairs)
	state.pairs = []

	const prefillPairs = prefill?.config.pairs ?? []

	// Same-asset cross-chain transfer markets.
	for (const symbol of ["USDC", "USDT"]) {
		const existing = prefillPairs.find(
			(p) => normalizeSymbol(p.token0) === symbol && normalizeSymbol(p.token1) === symbol,
		)
		const enable = guard(
			await confirm({
				message: `Run the ${symbol} -> ${symbol} cross-chain transfer market?`,
				// Fresh runs default on; update runs default to what the config had.
				initialValue: prefill ? Boolean(existing) : true,
			}),
		)
		if (!enable) continue
		state.pairs.push({ token0: symbol, token1: symbol })
	}

	// Further markets: any pair of assets (stable/exotic, stable/stable, exotic/exotic),
	// including non-USDC/USDT same-token transfer markets from the prefill.
	const otherPairs = prefillPairs.filter(
		(p) =>
			!(
				normalizeSymbol(p.token0) === normalizeSymbol(p.token1) &&
				["USDC", "USDT"].includes(normalizeSymbol(p.token0))
			),
	)
	let crossIndex = 0
	let addCross = guard(
		await confirm({
			message: "Add another market (any asset pair, e.g. USDC/CNGN, USDC/USDT, ZARP/CNGN)?",
			initialValue: otherPairs.length > 0,
		}),
	)
	while (addCross) {
		const pair = await buildMarketPair(state, otherPairs[crossIndex], prefill)
		// A market has one orientation — the reverse is the same book from the other side.
		const clashes = state.pairs.some(
			(p) =>
				(normalizeSymbol(p.token0) === normalizeSymbol(pair.token0) &&
					normalizeSymbol(p.token1) === normalizeSymbol(pair.token1)) ||
				(normalizeSymbol(p.token0) === normalizeSymbol(pair.token1) &&
					normalizeSymbol(p.token1) === normalizeSymbol(pair.token0)),
		)
		if (clashes) {
			log.error(
				`${pair.token0}/${pair.token1} is already declared (a pair and its reverse are the same market) — skipping.`,
			)
		} else {
			state.pairs.push(pair)
		}
		crossIndex += 1
		addCross = guard(
			await confirm({ message: "Add another market?", initialValue: crossIndex < otherPairs.length }),
		)
	}

	if (state.pairs.length === 0) {
		log.error("At least one pair is required — enabling the USDC transfer market with defaults.")
		state.pairs.push({ token0: "USDC", token1: "USDC" })
	}

	if ((prefill?.config.vault as { uniswapV4?: unknown } | undefined)?.uniswapV4) {
		log.info("Dropping the existing Uniswap V4 positions — pool pricing and pool funding are no longer supported.")
	}

	applyTestnetConfirmationPolicies(state, prefill)
}

async function buildMarketPair(
	state: WizardState,
	existing: PairConfig | undefined,
	prefill?: Prefill,
): Promise<PairConfig> {
	// A prefilled symbol outside the registry list (a custom [assets] token) is
	// prepended as its own option so the existing value round-trips instead of
	// clack silently falling back to the first option.
	const symbolOptions = (existingSymbol?: string) => {
		const options = [
			...SYMBOL_OPTIONS.map((symbol) => ({ value: symbol, label: symbol })),
			{ value: "custom", label: "Custom token…", hint: "an asset the registry doesn't ship" },
		]
		if (existingSymbol && !SYMBOL_OPTIONS.includes(existingSymbol)) {
			options.unshift({ value: existingSymbol, label: `${existingSymbol} (custom)` })
		}
		return options
	}

	const existingToken0 = existing ? normalizeSymbol(existing.token0) : undefined
	const token0Choice = guard(
		await select({
			message: "Quote asset (token0)",
			initialValue: existingToken0 ?? "USDC",
			options: symbolOptions(existingToken0),
		}),
	)
	const token0 = token0Choice === "custom" ? await addCustomAsset(state) : token0Choice

	const existingToken1 = existing ? normalizeSymbol(existing.token1) : undefined
	const token1Choice = guard(
		await select({
			message: "Base asset (token1) — the token you market-make against the quote",
			initialValue: existingToken1 ?? "CNGN",
			options: symbolOptions(existingToken1),
		}),
	)
	const token1 = token1Choice === "custom" ? await addCustomAsset(state) : token1Choice

	return { token0, token1 }
}

async function addCustomAsset(state: WizardState): Promise<string> {
	if (state.chains.length === 0) throw new Error("At least one managed chain is required for this step")
	const symbol = normalizeSymbol(
		await askText("Token symbol (used in the config, e.g. BRZ)", {
			required: "Symbol is required",
			validate: (value) => {
				if (!/^[A-Za-z0-9_-]+$/.test(value.trim())) return "Letters/digits only"
				// A custom address under a shipped symbol would silently repoint
				// the real asset — the same block the web wizard applies.
				if (isRegistrySymbol(value)) {
					return `${normalizeSymbol(value)} ships with the registry — pick it from the list instead of redefining its address`
				}
				return undefined
			},
		}),
	)
	const addresses: Record<string, HexString> = {}
	while (Object.keys(addresses).length === 0) {
		for (const chain of state.chains) {
			const hasToken = guard(
				await confirm({ message: `Does ${symbol} exist on ${chain.meta.label}?`, initialValue: false }),
			)
			if (!hasToken) continue
			addresses[chain.meta.stateMachineId] = (await askAddress(
				`${symbol} address on ${chain.meta.label}`,
			)) as HexString
		}
		if (Object.keys(addresses).length === 0) {
			log.error(`${symbol} needs an address on at least one selected chain.`)
		}
	}
	state.assets = { ...(state.assets ?? {}), [symbol]: addresses }
	return symbol
}

/**
 * Testnet chain ids have no built-in confirmation defaults, so an explicit
 * low-value policy is always written for them. Policies are top-level, keyed
 * by chain id; prefilled ones are carried and testnet gaps filled in.
 */
function applyTestnetConfirmationPolicies(state: WizardState, prefill?: Prefill): void {
	const carried = normalizeConfirmationPolicyKeys(prefill?.config.confirmationPolicies ?? {})
	const policies: Record<string, ChainConfirmationPolicy> = { ...carried }
	if (state.network === "testnet") {
		for (const chain of state.chains) {
			policies[String(chain.meta.chainId)] ??= { points: TESTNET_CONFIRMATION_POINTS }
		}
	}
	state.confirmationPolicies = Object.keys(policies).length > 0 ? policies : undefined
}
