import { confirm, log, select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import type { UniswapV4PositionToml, ChainConfirmationPolicy } from "@/config/filler-toml"
import { unanchoredToken0Symbols, type PairConfig } from "@/config/pairs"
import { pickAnchorStable } from "@/config/wizard-guards"
import { isRegistrySymbol, normalizeSymbol, registrySymbols, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import { bookCrossedAt, type PriceCurvePoint } from "@/config/interpolated-curve"
import { guard, why, askText, askNumber, askAddress } from "../prompt-utils"
import { editPoints, positiveValue } from "../points-editor"
import { normalizeConfirmationPolicyKeys } from "../confirmation-keys"
import { WHY } from "../help-text"
import {
	DEFAULT_SAME_ASSET_ASK_CURVE,
	TESTNET_CONFIRMATION_POINTS,
	type Prefill,
	type WizardState,
} from "../state"

/** Registry symbols with USD stables first — the built-in candidates for either side. */
const SYMBOL_OPTIONS = [
	...registrySymbols().filter((symbol) => USD_STABLE_SYMBOLS.has(symbol)),
	...registrySymbols().filter((symbol) => !USD_STABLE_SYMBOLS.has(symbol)),
]

/** Near-par two-sided defaults for stable-to-stable markets. */
const STABLE_SWAP_BID = [{ amount: "0", price: "1.001" }]
const STABLE_SWAP_ASK = [{ amount: "0", price: "0.999" }]

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

	// Reference-only pairs are price feeds, not markets — re-offering them
	// through the market prompts would drop the flag; carry them, but through
	// the same checks market pairs get, so a broken prefill feed is dropped
	// here with a reason instead of failing the write gate and wiping the step.
	const referencePairs = prefillPairs.filter((p) => p.referenceOnly === true)
	const kept: PairConfig[] = []
	for (const pair of referencePairs) {
		const issue = referencePairIssue(state.pairs.concat(kept), pair)
		if (issue) {
			log.warn(`Dropping reference-only ${pair.token0}/${pair.token1} from the existing config: ${issue}`)
			continue
		}
		kept.push({ ...pair })
	}
	if (kept.length > 0) {
		state.pairs.push(...kept)
		log.info(
			`Keeping ${kept.length} reference-only price feed${kept.length > 1 ? "s" : ""}: ${kept.map((p) => `${p.token0}/${p.token1}`).join(", ")}`,
		)
	}

	// Further markets: any pair of assets (stable/exotic, stable/stable, exotic/exotic),
	// including non-USDC/USDT same-token transfer markets from the prefill.
	const otherPairs = prefillPairs.filter(
		(p) =>
			p.referenceOnly !== true &&
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
		state.pairs.push({
			token0: "USDC",
			token1: "USDC",
			maxOrderSize: "100000",
			askPriceCurve: [...DEFAULT_SAME_ASSET_ASK_CURVE],
		})
	}

	const prefillUniswapV4 = prefill?.config.vault?.uniswapV4
	if (prefillUniswapV4 && !state.vaultUniswapV4) {
		const remove = guard(
			await confirm({
				message: "The existing Uniswap V4 positions are no longer used by any market — remove them from the config?",
				initialValue: true,
			}),
		)
		if (!remove) state.vaultUniswapV4 = JSON.parse(JSON.stringify(prefillUniswapV4))
	}
	await chooseVenueSide(state, prefill)

	await ensureUsdAnchors(state)
	applyTestnetConfirmationPolicies(state, prefill)
}

/**
 * One-sided pool pricing (`side`) is only valid when no pair carries a static
 * curve — curves express one-sidedness by omission, and the engine rejects the
 * combination. Offer the select only when it can actually be written.
 */
async function chooseVenueSide(state: WizardState, prefill?: Prefill): Promise<void> {
	const venue = state.vaultUniswapV4
	if (!venue) return
	const anyCurves = state.pairs.some(
		(p) => (p.bidPriceCurve?.length ?? 0) > 0 || (p.askPriceCurve?.length ?? 0) > 0,
	)
	if (anyCurves) {
		if (venue.side !== undefined) delete venue.side
		log.info(
			"One-sided pool pricing is unavailable: some pairs carry static curves (curves express one-sidedness by omission) — pool pricing fills both directions.",
		)
		return
	}
	const side = guard(
		await select({
			message: "Which directions should pool pricing fill?",
			initialValue: (prefill?.config.vault?.uniswapV4?.side as string) ?? "both",
			options: [
				{ value: "both", label: "Both directions" },
				{ value: "ask", label: "Ask only", hint: "sell the base token, accumulate the quote" },
				{ value: "bid", label: "Bid only", hint: "buy the base token, accumulate it" },
			],
		}),
	)
	if (side === "both") delete venue.side
	else venue.side = side as "bid" | "ask"
}

/** Why a prefilled reference-only pair cannot be carried, or null when it can. */
function referencePairIssue(existing: PairConfig[], pair: PairConfig): string | null {
	if (normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)) {
		return "a same-token pair carries no FX rate to reference"
	}
	if ((pair.bidPriceCurve?.length ?? 0) < 1 && (pair.askPriceCurve?.length ?? 0) < 1) {
		return "it has no price curve (the curve IS the reference)"
	}
	const clashes = existing.some(
		(p) =>
			(normalizeSymbol(p.token0) === normalizeSymbol(pair.token0) &&
				normalizeSymbol(p.token1) === normalizeSymbol(pair.token1)) ||
			(normalizeSymbol(p.token0) === normalizeSymbol(pair.token1) &&
				normalizeSymbol(p.token1) === normalizeSymbol(pair.token0)),
	)
	if (clashes) return "that market is already declared (a pair and its reverse are the same market)"
	return null
}

/**
 * Every pair's token0 must reach a USD anchor through the declared curves —
 * the engine refuses the config otherwise. Offer a reference-only price feed
 * against a USD stable for each unanchored symbol; declining means the config
 * is rejected at the final write gate and the wizard brings the operator back
 * to this step.
 */
async function ensureUsdAnchors(state: WizardState): Promise<void> {
	for (;;) {
		const unanchored = unanchoredToken0Symbols(
			state.pairs.map((p) => ({
				token0: p.token0,
				token1: p.token1,
				hasCurve: (p.bidPriceCurve?.length ?? 0) >= 1 || (p.askPriceCurve?.length ?? 0) >= 1,
			})),
		)
		if (unanchored.length === 0) return

		const symbol = unanchored[0]
		const stable = pickAnchorStable(state.pairs, symbol)
		if (!stable) {
			log.warn(
				`${symbol} cannot be auto-anchored: every USD stable already has a pair against it, but none carries a curve (a venue-priced pair has no rate to anchor with). Add a curve-priced pair against a USD stable by hand — until then the config is rejected at the final check and you'll be brought back to this step.`,
			)
			break
		}
		why(WHY.anchor)
		const addFeed = guard(
			await confirm({
				message: `No USD anchor for ${symbol} — add a reference-only ${stable}/${symbol} price feed? (anchors it without opening that market)`,
				initialValue: true,
			}),
		)
		if (!addFeed) {
			log.warn(
				`Without an anchor for ${symbol} the config will be rejected at the final check and you'll be brought back to this step.`,
			)
			return
		}
		const price = await askText(`Reference price (${symbol} per ${stable}, roughly current)`, {
			required: "Reference price is required",
			validate: (value) =>
				Number.isFinite(Number(value)) && Number(value) > 0 ? undefined : "Enter a positive number",
		})
		state.pairs.push({
			token0: stable,
			token1: symbol,
			referenceOnly: true,
			askPriceCurve: [{ amount: "0", price }],
		})
	}
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
			message: "Quote asset (token0) — curves and the order cap are denominated in it",
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

	why(WHY.maxOrderSize)
	const maxOrderSize = await askNumber(
		`Maximum ${token0} per order`,
		Number(existing?.maxOrderSize ?? 5000),
		(n) => (n > 0 ? undefined : "Enter a positive number"),
	)

	const pair: PairConfig = { token0, token1, maxOrderSize: String(maxOrderSize) }

	// Same asset on both sides: a cross-chain transfer market, ask-only below par.
	if (normalizeSymbol(token0) === normalizeSymbol(token1)) {
		why(WHY.sameAssetCurve)
		pair.askPriceCurve = await editPoints<PriceCurvePoint>({
			prompt: `Ask point as \`orderSize,price\` (price below 1, e.g. \`1000,0.995\`); empty line to finish`,
			minPoints: 1,
			checkValue: belowParPrice,
			initial: existing?.askPriceCurve ?? DEFAULT_SAME_ASSET_ASK_CURVE,
			toPoint: ({ first, second }) => ({ amount: first, price: second }),
		})
		return pair
	}

	// Pool pricing only quotes markets with a USD-stable quote asset.
	const venueAllowed = USD_STABLE_SYMBOLS.has(normalizeSymbol(token0))
	let pricingSource = "curves"
	if (venueAllowed) {
		why(WHY.fxPricing)
		pricingSource = guard(
			await select({
				message: `Price source for ${token0}/${token1}`,
				// Venue pricing is the default only for a pair that had no curves
				// at all — an ask-only static pair must not flip to the venue.
				initialValue:
					!(existing?.bidPriceCurve?.length || existing?.askPriceCurve?.length) &&
					prefill?.config.vault?.uniswapV4?.positions?.length
						? "uniswapV4"
						: "curves",
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
	} else {
		log.info(`Pool pricing needs a USD-stable quote asset — ${token0}/${token1} uses static curves.`)
	}

	if (pricingSource === "curves") {
		await editCrossAssetCurves(pair, existing)
	} else {
		await configureUniswapV4(state, prefill)
	}
	return pair
}

/** Prompts for bid/ask curves until at least one side is set; warns on a crossed book. */
async function editCrossAssetCurves(pair: PairConfig, existing?: PairConfig): Promise<void> {
	// Stable-to-stable markets get near-par starting points; buy at a premium,
	// sell at a discount, book uncrossed by construction.
	const bothStable =
		USD_STABLE_SYMBOLS.has(normalizeSymbol(pair.token0)) && USD_STABLE_SYMBOLS.has(normalizeSymbol(pair.token1))
	const bidInitial = existing?.bidPriceCurve ?? (bothStable ? STABLE_SWAP_BID : undefined)
	const askInitial = existing?.askPriceCurve ?? (bothStable ? STABLE_SWAP_ASK : undefined)
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
				initial: bidInitial,
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
				initial: askInitial,
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
			const crossed = bookCrossedAt(pair.bidPriceCurve, pair.askPriceCurve)
			if (crossed) {
				log.warn(
					`${pair.token0}/${pair.token1}: book is crossed at amount ${crossed.amount} (bid ${crossed.bid} ≤ ask ${crossed.ask}) — both sides still fill at their own curve, but a full round trip at these prices loses money. Leave it only if deliberate.`,
				)
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

/** Collects the top-level [vault.uniswapV4] venue block (positions, spread; side is chosen after the market loop). */
async function configureUniswapV4(state: WizardState, prefill?: Prefill): Promise<void> {
	if (state.vaultUniswapV4) return // already configured by an earlier pair this run

	const existing = prefill?.config.vault?.uniswapV4
	const positions: UniswapV4PositionToml[] = []
	for (const position of existing?.positions ?? []) {
		const guardLabel = position.referencePrice
			? `guard ${position.referencePrice} ±${position.maxDeviationBps} bps`
			: "no price guard"
		const keep = guard(
			await confirm({
				message: `Keep position ${position.tokenId} on ${position.chain} (${guardLabel})?`,
				initialValue: true,
			}),
		)
		if (keep) positions.push({ ...position })
	}
	if (positions.length === 0) positions.push(await askPosition(state))
	while (guard(await confirm({ message: "Add another Uniswap V4 position?", initialValue: false }))) {
		positions.push(await askPosition(state))
	}

	state.vaultUniswapV4 = {
		positions,
		...(existing?.spreadBps !== undefined ? { spreadBps: existing.spreadBps } : {}),
	}
}

async function askPosition(state: WizardState): Promise<UniswapV4PositionToml> {
	if (state.chains.length === 0) throw new Error("At least one managed chain is required for this step")
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
			validate: (value) =>
				Number.isFinite(Number(value)) && Number(value) > 0 ? undefined : "Enter a positive number",
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
	const carried = normalizeConfirmationPolicyKeys(prefill?.config.confirmationPolicies ?? {})
	const policies: Record<string, ChainConfirmationPolicy> = { ...carried }
	if (state.network === "testnet") {
		for (const chain of state.chains) {
			policies[String(chain.meta.chainId)] ??= { points: TESTNET_CONFIRMATION_POINTS }
		}
	}
	state.confirmationPolicies = Object.keys(policies).length > 0 ? policies : undefined
}
