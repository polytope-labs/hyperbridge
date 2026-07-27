import { confirm, log, select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import type { UniswapV4PositionToml, ChainConfirmationPolicy } from "@/config/filler-toml"
import type { PairConfig } from "@/config/pairs"
import { normalizeSymbol, registrySymbols, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import { FillerPricePolicy, type PriceCurvePoint } from "@/config/interpolated-curve"
import { guard, why, askText, askNumber, askAddress } from "../prompt-utils"
import { editPoints, positiveValue } from "../points-editor"
import { WHY } from "../help-text"
import {
	DEFAULT_SAME_ASSET_ASK_CURVE,
	TESTNET_CONFIRMATION_POINTS,
	type Prefill,
	type WizardState,
} from "../state"

/** Registry symbols that are not USD stables — the built-in cross-asset candidates. */
const EXOTIC_SYMBOLS = registrySymbols().filter((symbol) => !USD_STABLE_SYMBOLS.has(symbol))

const belowParPrice = (value: number): string | undefined =>
	value > 0 && value < 1 ? undefined : "Must be between 0 and 1 (exclusive) — the gap to 1 is your spread"

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

		why(WHY.maxOrderSize)
		const maxOrderSize = await askNumber(
			`Maximum ${symbol} per order`,
			Number(existing?.maxOrderSize ?? 100_000),
			(n) => (n > 0 ? undefined : "Enter a positive number"),
		)
		why(WHY.sameAssetCurve)
		const askPriceCurve = await editPoints<PriceCurvePoint>({
			prompt: `Ask point as \`orderSize,price\` (price below 1, e.g. \`1000,0.995\`); empty line to finish`,
			minPoints: 1,
			checkValue: belowParPrice,
			initial: existing?.askPriceCurve ?? DEFAULT_SAME_ASSET_ASK_CURVE,
			toPoint: ({ first, second }) => ({ amount: first, price: second }),
		})
		state.pairs.push({ token0: symbol, token1: symbol, maxOrderSize: String(maxOrderSize), askPriceCurve })
	}

	// Cross-asset FX markets.
	const existingCross = prefillPairs.filter((p) => normalizeSymbol(p.token0) !== normalizeSymbol(p.token1))
	let crossIndex = 0
	let addCross = guard(
		await confirm({
			message: "Add a cross-asset FX market (e.g. USDC/CNGN)?",
			initialValue: existingCross.length > 0,
		}),
	)
	while (addCross) {
		state.pairs.push(await buildCrossAssetPair(state, existingCross[crossIndex], prefill))
		crossIndex += 1
		addCross = guard(
			await confirm({ message: "Add another cross-asset market?", initialValue: crossIndex < existingCross.length }),
		)
	}

	if (state.pairs.length === 0) {
		log.error("At least one pair is required — enabling the USDC transfer market with defaults.")
		state.pairs.push({
			token0: "USDC",
			token1: "USDC",
			maxOrderSize: "100000",
			askPriceCurve: [...DEFAULT_SAME_ASSET_ASK_CURVE],
		})
	}

	applyTestnetConfirmationPolicies(state, prefill)
}

async function buildCrossAssetPair(
	state: WizardState,
	existing: PairConfig | undefined,
	prefill?: Prefill,
): Promise<PairConfig> {
	const token0 = guard(
		await select({
			message: "Quote asset (token0) — curves and the order cap are denominated in it",
			initialValue: existing ? normalizeSymbol(existing.token0) : "USDC",
			options: [...USD_STABLE_SYMBOLS].map((symbol) => ({ value: symbol, label: symbol })),
		}),
	)

	const token1Choice = guard(
		await select({
			message: "Base asset (token1) — the token you market-make against the quote",
			initialValue:
				existing && EXOTIC_SYMBOLS.includes(normalizeSymbol(existing.token1))
					? normalizeSymbol(existing.token1)
					: EXOTIC_SYMBOLS[0],
			options: [
				...EXOTIC_SYMBOLS.map((symbol) => ({ value: symbol, label: symbol })),
				{ value: "custom", label: "Custom token…", hint: "an asset the registry doesn't ship" },
			],
		}),
	)
	const token1 = token1Choice === "custom" ? await addCustomAsset(state) : token1Choice

	why(WHY.maxOrderSize)
	const maxOrderSize = await askNumber(
		`Maximum ${token0} per order`,
		Number(existing?.maxOrderSize ?? 5000),
		(n) => (n > 0 ? undefined : "Enter a positive number"),
	)

	const pair: PairConfig = { token0, token1, maxOrderSize: String(maxOrderSize) }

	why(WHY.fxPricing)
	const pricingSource = guard(
		await select({
			message: `Price source for ${token0}/${token1}`,
			initialValue: prefill?.config.vault?.uniswapV4?.positions?.length && !existing?.bidPriceCurve ? "uniswapV4" : "curves",
			options: [
				{ value: "curves", label: "Static bid/ask curves", hint: "you maintain the prices" },
				{
					value: "uniswapV4",
					label: "Uniswap V4 LP positions",
					hint: "pool price is the oracle; also funds fills",
				},
			],
		}),
	)

	if (pricingSource === "curves") {
		await editCrossAssetCurves(pair, existing)
	} else {
		await configureUniswapV4(state, prefill)
	}
	return pair
}

/** Prompts for bid/ask curves until the book is valid (uncrossed, ≥1 side). */
async function editCrossAssetCurves(pair: PairConfig, existing?: PairConfig): Promise<void> {
	for (;;) {
		why(WHY.crossAssetCurves)
		const withBid = guard(
			await confirm({
				message: `Fill ${pair.token1} -> ${pair.token0} orders (buy ${pair.token1})? Requires a bid curve.`,
				initialValue: existing ? Boolean(existing.bidPriceCurve?.length) : true,
			}),
		)
		if (withBid) {
			pair.bidPriceCurve = await editPoints<PriceCurvePoint>({
				prompt: `Bid point as \`orderSize,price\` (${pair.token1} per ${pair.token0} when buying); empty line to finish`,
				minPoints: 1,
				checkValue: positiveValue,
				initial: existing?.bidPriceCurve,
				toPoint: ({ first, second }) => ({ amount: first, price: second }),
			})
		} else {
			pair.bidPriceCurve = undefined
		}
		const withAsk = guard(
			await confirm({
				message: `Fill ${pair.token0} -> ${pair.token1} orders (sell ${pair.token1})? Requires an ask curve.`,
				initialValue: existing ? Boolean(existing.askPriceCurve?.length) : true,
			}),
		)
		if (withAsk) {
			pair.askPriceCurve = await editPoints<PriceCurvePoint>({
				prompt: `Ask point as \`orderSize,price\` (${pair.token1} per ${pair.token0} when selling); empty line to finish`,
				minPoints: 1,
				checkValue: positiveValue,
				initial: existing?.askPriceCurve,
				toPoint: ({ first, second }) => ({ amount: first, price: second }),
			})
		} else {
			pair.askPriceCurve = undefined
		}

		if (!withBid && !withAsk) {
			log.error("At least one direction is required.")
			continue
		}
		if (withBid && !withAsk) {
			log.warn(`One-sided LP: the filler only buys ${pair.token1} and accumulates it.`)
		}
		if (withAsk && !withBid) {
			log.warn(`One-sided LP: the filler only sells ${pair.token1} and accumulates ${pair.token0}.`)
		}
		if (pair.bidPriceCurve?.length && pair.askPriceCurve?.length) {
			try {
				FillerPricePolicy.assertBookNotCrossed(
					`${pair.token0}/${pair.token1}`,
					new FillerPricePolicy({ points: pair.bidPriceCurve }),
					new FillerPricePolicy({ points: pair.askPriceCurve }),
				)
			} catch (err) {
				log.error(`${err instanceof Error ? err.message : err}\nRe-enter the curves.`)
				continue
			}
		}
		return
	}
}

/**
 * Registers a custom asset in the `[assets]` table: symbol plus its contract
 * address on every selected chain it exists on.
 */
async function addCustomAsset(state: WizardState): Promise<string> {
	const symbol = normalizeSymbol(
		await askText("Token symbol (used in the config, e.g. BRZ)", {
			required: "Symbol is required",
			validate: (value) => (/^[A-Za-z0-9_-]+$/.test(value.trim()) ? undefined : "Letters/digits only"),
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

/** Collects the top-level [vault.uniswapV4] venue block (positions, side, spread). */
async function configureUniswapV4(state: WizardState, prefill?: Prefill): Promise<void> {
	if (state.vaultUniswapV4) return // already configured by an earlier pair this run

	const existing = prefill?.config.vault?.uniswapV4
	const positions: UniswapV4PositionToml[] = []
	do {
		positions.push(await askPosition(state))
	} while (guard(await confirm({ message: "Add another Uniswap V4 position?", initialValue: false })))

	const side = guard(
		await select({
			message: "Which directions should pool pricing fill?",
			initialValue: (existing?.side as string) ?? "both",
			options: [
				{ value: "both", label: "Both directions" },
				{ value: "ask", label: "Ask only", hint: "sell the base token, accumulate the quote" },
				{ value: "bid", label: "Bid only", hint: "buy the base token, accumulate it" },
			],
		}),
	)
	state.vaultUniswapV4 = {
		positions,
		...(side === "both" ? {} : { side: side as "bid" | "ask" }),
		...(existing?.spreadBps !== undefined ? { spreadBps: existing.spreadBps } : {}),
	}
}

async function askPosition(state: WizardState): Promise<UniswapV4PositionToml> {
	const chain = guard(
		await select({
			message: "Chain the position lives on",
			options: state.chains.map((c) => ({ value: c.meta.stateMachineId, label: c.meta.label })),
		}),
	)
	const tokenId = await askText("Position token ID (from the position's URL)", {
		required: "Token id is required",
		validate: (value) => (/^\d+$/.test(value) ? undefined : "Enter a numeric token id"),
	})
	const position: UniswapV4PositionToml = { chain, tokenId }

	const withGuardPrice = guard(
		await confirm({
			message: "Add a price guard? (rejects fills when the pool drifts from a reference price)",
			initialValue: false,
		}),
	)
	if (withGuardPrice) {
		position.referencePrice = await askText("Reference price (base tokens per quote token)", {
			required: "Reference price is required",
			validate: (value) => (Number(value) > 0 ? undefined : "Enter a positive number"),
		})
		position.maxDeviationBps = await askNumber(
			"Maximum deviation from the reference (basis points, e.g. 200 = 2%)",
			200,
			(parsed) => (parsed > 0 && parsed <= 10_000 ? undefined : "Enter a number between 1 and 10000"),
		)
	}
	return position
}

/**
 * Testnet chain ids have no built-in confirmation defaults, so an explicit
 * low-value policy is always written for them. Policies are top-level, keyed
 * by chain id; prefilled ones are carried and testnet gaps filled in.
 */
function applyTestnetConfirmationPolicies(state: WizardState, prefill?: Prefill): void {
	const carried = prefill?.config.confirmationPolicies ?? {}
	const policies: Record<string, ChainConfirmationPolicy> = { ...carried }
	if (state.network === "testnet") {
		for (const chain of state.chains) {
			policies[String(chain.meta.chainId)] ??= { points: TESTNET_CONFIRMATION_POINTS }
		}
	}
	state.confirmationPolicies = Object.keys(policies).length > 0 ? policies : undefined
}
