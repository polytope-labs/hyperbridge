import { confirm, log, select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import type { FxStrategyConfig, StrategyConfig, ChainConfirmationPolicy } from "@/config/filler-toml"
import type { UniswapV4PositionToml } from "@/config/filler-toml"
import { guard, why, askText, askNumber, askAddress } from "../prompt-utils"
import { editPoints, positiveValue } from "../points-editor"
import { WHY } from "../help-text"
import { TESTNET_CONFIRMATION_POINTS, type Prefill, type WizardState } from "../state"

export async function stepStrategies(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.strategies)
	// The wizard only configures HyperFX. Strategies of other types in an
	// existing config (e.g. stable) are carried through update runs verbatim.
	const carried = prefill?.config.strategies.filter((s) => s.type !== "hyperfx") ?? []
	state.strategies = [...carried]

	const configureFx =
		carried.length === 0
			? true
			: guard(
					await confirm({
						message: "Configure the HyperFX strategy (stablecoin <-> exotic token, e.g. cNGN)?",
						initialValue: prefill?.config.strategies.some((s) => s.type === "hyperfx") ?? true,
					}),
				)
	if (configureFx) {
		state.strategies.push(await buildFxStrategy(state, prefill))
	}
}

async function buildFxStrategy(state: WizardState, prefill?: Prefill): Promise<FxStrategyConfig> {
	const existing = prefill?.config.strategies.find((s): s is FxStrategyConfig => s.type === "hyperfx")

	why(WHY.maxOrderUsd)
	const maxOrderUsd = await askNumber("Maximum USD value per order", existing?.maxOrderUsd ?? 5000, (n) =>
		n > 0 ? undefined : "Enter a positive number",
	)

	why(WHY.token1)
	const token1: Record<string, HexString> = {}
	while (Object.keys(token1).length === 0) {
		for (const chain of state.chains) {
			const key = chain.meta.stateMachineId
			const previous = existing?.token1?.[key]
			const hasToken = guard(
				await confirm({
					message: `Does the exotic token exist on ${chain.meta.label}?`,
					initialValue: previous !== undefined,
				}),
			)
			if (!hasToken) continue
			token1[key] = (await askAddress(`Exotic token address on ${chain.meta.label}`, {
				initial: previous,
			})) as HexString
		}
		if (Object.keys(token1).length === 0) {
			log.error("HyperFX needs the exotic token on at least one selected chain.")
		}
	}

	why(WHY.fxPricing)
	const pricingSource = guard(
		await select({
			message: "Price source for the exotic token",
			initialValue: existing?.vault?.uniswapV4?.positions?.length ? "uniswapV4" : "curves",
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

	const strategy: FxStrategyConfig = { type: "hyperfx", maxOrderUsd, token1 }

	if (pricingSource === "curves") {
		const withBid = guard(
			await confirm({
				message: "Fill exotic -> stable orders (buy exotic)? Requires a bid curve.",
				initialValue: existing ? Boolean(existing.bidPriceCurve?.length) : true,
			}),
		)
		if (withBid) {
			strategy.bidPriceCurve = await editPoints({
				prompt: "Bid point as `orderUsd,exoticPerUsd` (price when buying exotic); empty line to finish",
				minPoints: 1,
				checkValue: positiveValue,
				initial: existing?.bidPriceCurve,
				toPoint: ({ first, second }) => ({ amount: first, price: second }),
			})
		}
		const withAsk = guard(
			await confirm({
				message: "Fill stable -> exotic orders (sell exotic)? Requires an ask curve.",
				initialValue: existing ? Boolean(existing.askPriceCurve?.length) : true,
			}),
		)
		if (withAsk) {
			strategy.askPriceCurve = await editPoints({
				prompt: "Ask point as `orderUsd,exoticPerUsd` (price when selling exotic); empty line to finish",
				minPoints: 1,
				checkValue: positiveValue,
				initial: existing?.askPriceCurve,
				toPoint: ({ first, second }) => ({ amount: first, price: second }),
			})
		}
		if (!withBid && !withAsk) {
			log.error("At least one direction is required — enabling both curves.")
			return buildFxStrategy(state, prefill)
		}
		if (!withBid || !withAsk) {
			log.warn(
				"One-sided LP: the filler only trades in the direction you configured and accumulates the other token.",
			)
		}
	} else {
		const positions: UniswapV4PositionToml[] = []
		do {
			positions.push(await askPosition(state))
		} while (guard(await confirm({ message: "Add another Uniswap V4 position?", initialValue: false })))

		const side = guard(
			await select({
				message: "Which directions should pool pricing fill?",
				initialValue: (existing?.vault?.uniswapV4?.side as string) ?? "both",
				options: [
					{ value: "both", label: "Both directions" },
					{ value: "ask", label: "Ask only", hint: "sell exotic, accumulate stablecoins" },
					{ value: "bid", label: "Bid only", hint: "buy exotic, accumulate the exotic token" },
				],
			}),
		)
		strategy.vault = {
			uniswapV4: {
				positions,
				...(side === "both" ? {} : { side: side as "bid" | "ask" }),
			},
		}
	}

	applyTestnetConfirmationPolicies(state, strategy)
	if (existing?.confirmationPolicies) {
		strategy.confirmationPolicies = { ...existing.confirmationPolicies, ...strategy.confirmationPolicies }
	}
	return strategy
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
		position.referencePrice = await askText("Reference price (exotic tokens per USD)", {
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
 * low-value policy is always written for them.
 */
function applyTestnetConfirmationPolicies(state: WizardState, strategy: StrategyConfig): void {
	if (state.network !== "testnet") return
	const policies: Record<string, ChainConfirmationPolicy> = {}
	for (const chain of state.chains) {
		policies[String(chain.meta.chainId)] = { points: TESTNET_CONFIRMATION_POINTS }
	}
	strategy.confirmationPolicies = policies
}
