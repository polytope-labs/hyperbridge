import { confirm, log, multiselect, note, select, text } from "@clack/prompts"
import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type {
	FxStrategyConfig,
	StableStrategyConfig,
	StrategyConfig,
	ChainConfirmationPolicy,
} from "@/config/filler-toml"
import type { UniswapV4PositionToml } from "@/config/filler-toml"
import { guard, why, parsePointInput } from "../prompt-utils"
import { WHY } from "../help-text"
import { DEFAULT_STABLE_BPS_CURVE, TESTNET_CONFIRMATION_POINTS, type Prefill, type WizardState } from "../state"

export async function stepStrategies(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.strategies)
	const existingTypes = new Set(prefill?.config.strategies.map((s) => s.type) ?? [])

	const selected = guard(
		await multiselect<"stable" | "hyperfx">({
			message: "Which strategies do you want to run?",
			options: [
				{ value: "stable", label: "Stable", hint: "USDC->USDC / USDT->USDT across chains" },
				{ value: "hyperfx", label: "HyperFX", hint: "stablecoin <-> exotic token (e.g. cNGN)" },
			],
			initialValues: existingTypes.size > 0 ? [...existingTypes] : ["stable"],
			required: true,
		}),
	)

	state.strategies = []
	if (selected.includes("stable")) {
		state.strategies.push(await buildStableStrategy(state, prefill))
	}
	if (selected.includes("hyperfx")) {
		state.strategies.push(await buildFxStrategy(state, prefill))
	}
}

async function buildStableStrategy(state: WizardState, prefill?: Prefill): Promise<StableStrategyConfig> {
	const existing = prefill?.config.strategies.find((s): s is StableStrategyConfig => s.type === "stable")
	const defaultCurve = existing?.bpsCurve ?? DEFAULT_STABLE_BPS_CURVE

	why(WHY.bpsCurve)
	note(
		defaultCurve.map((point) => `$${point.amount} orders -> ${point.value} bps (${point.value / 100}%)`).join("\n"),
		"Margin curve",
	)
	const useDefault = guard(
		await confirm({ message: "Use this margin curve? (you can edit points instead)", initialValue: true }),
	)

	const bpsCurve = useDefault
		? defaultCurve
		: await editPoints({
				prompt: "Curve point as `orderUsd,bps` (e.g. `1000,50`); empty line to finish",
				minPoints: 2,
				toPoint: ({ first, second }) => ({ amount: first, value: Number(second) }),
			})

	const strategy: StableStrategyConfig = { type: "stable", bpsCurve }
	applyTestnetConfirmationPolicies(state, strategy)
	if (existing?.confirmationPolicies) {
		strategy.confirmationPolicies = { ...existing.confirmationPolicies, ...strategy.confirmationPolicies }
	}
	return strategy
}

async function buildFxStrategy(state: WizardState, prefill?: Prefill): Promise<FxStrategyConfig> {
	const existing = prefill?.config.strategies.find((s): s is FxStrategyConfig => s.type === "hyperfx")

	why(WHY.maxOrderUsd)
	const maxOrderUsd = guard(
		await text({
			message: "Maximum USD value per order",
			initialValue: existing ? String(existing.maxOrderUsd) : "5000",
			validate: (value) => (Number((value ?? "").trim()) > 0 ? undefined : "Enter a positive number"),
		}),
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
			const address = guard(
				await text({
					message: `Exotic token address on ${chain.meta.label}`,
					initialValue: previous,
					validate: (value) => (isAddress((value ?? "").trim()) ? undefined : "Enter a valid EVM address"),
				}),
			)
			token1[key] = address.trim() as HexString
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

	const strategy: FxStrategyConfig = {
		type: "hyperfx",
		maxOrderUsd: Number(maxOrderUsd.trim()),
		token1,
	}

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
	const tokenId = guard(
		await text({
			message: "Position token ID (from the position's URL)",
			validate: (value) => (/^\d+$/.test((value ?? "").trim()) ? undefined : "Enter a numeric token id"),
		}),
	)
	const position: UniswapV4PositionToml = { chain, tokenId: tokenId.trim() }

	const withGuardPrice = guard(
		await confirm({
			message: "Add a price guard? (rejects fills when the pool drifts from a reference price)",
			initialValue: false,
		}),
	)
	if (withGuardPrice) {
		const referencePrice = guard(
			await text({
				message: "Reference price (exotic tokens per USD)",
				validate: (value) => (Number((value ?? "").trim()) > 0 ? undefined : "Enter a positive number"),
			}),
		)
		const maxDeviationBps = guard(
			await text({
				message: "Maximum deviation from the reference (basis points, e.g. 200 = 2%)",
				validate: (value) => {
					const parsed = Number((value ?? "").trim())
					return parsed > 0 && parsed <= 10_000 ? undefined : "Enter a number between 1 and 10000"
				},
			}),
		)
		position.referencePrice = referencePrice.trim()
		position.maxDeviationBps = Number(maxDeviationBps.trim())
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

interface EditPointsOptions<P> {
	prompt: string
	minPoints: number
	initial?: P[]
	toPoint: (pair: { first: string; second: string }) => P
}

async function editPoints<P>(options: EditPointsOptions<P>): Promise<P[]> {
	const points: P[] = []
	if (options.initial?.length) {
		const keep = guard(
			await confirm({
				message: `Keep the ${options.initial.length} existing points and add more?`,
				initialValue: true,
			}),
		)
		if (keep) points.push(...options.initial)
	}
	for (;;) {
		const input = guard(
			await text({
				message: options.prompt,
				defaultValue: "",
				validate: (value) => {
					const trimmed = (value ?? "").trim()
					if (!trimmed) return undefined
					return parsePointInput(trimmed) ? undefined : "Expected two comma-separated numbers, e.g. `1000,50`"
				},
			}),
		)
		const trimmed = (input ?? "").trim()
		if (!trimmed) {
			if (points.length >= options.minPoints) return points
			log.error(`At least ${options.minPoints} point${options.minPoints > 1 ? "s" : ""} required.`)
			continue
		}
		points.push(options.toPoint(parsePointInput(trimmed)!))
	}
}
