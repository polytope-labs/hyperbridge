import { confirm, log, multiselect, note, select, text } from "@clack/prompts"
import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { DEFAULT_CONFIRMATION_POLICIES, type ChainConfirmationPolicy, type VaultToml } from "@/config/filler-toml"
import { guard, why, parsePointInput } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState } from "../state"

type Area =
	| "concurrency"
	| "gasFeeBump"
	| "overfill"
	| "confirmations"
	| "rebalancing"
	| "vault"
	| "allowlist"
	| "logging"

export async function stepFineTune(state: WizardState, prefill?: Prefill): Promise<void> {
	carryPrefillExtras(state, prefill)

	const tune = guard(
		await confirm({
			message: "Fine-tune the filler for competitiveness? (everything has working defaults)",
			initialValue: false,
		}),
	)
	if (!tune) return

	const areas = guard(
		await multiselect<Area>({
			message: "What do you want to tune?",
			required: false,
			options: [
				{ value: "concurrency", label: "Concurrency & retry queue", hint: WHY.concurrency },
				{ value: "gasFeeBump", label: "Gas fee bump", hint: "win more fill races" },
				{ value: "overfill", label: "Overfill protection", hint: "pricing-bug safety clamp" },
				{ value: "confirmations", label: "Confirmation policies", hint: "reorg protection per chain" },
				{ value: "rebalancing", label: "Rebalancing", hint: "auto top-up chains from richer ones" },
				{ value: "vault", label: "ERC-4626 treasury", hint: "earn yield on idle float" },
				{ value: "allowlist", label: "User allowlist", hint: "fill only for specific users" },
				{ value: "logging", label: "Log level" },
			],
		}),
	)

	for (const area of areas) {
		switch (area) {
			case "concurrency":
				await tuneConcurrency(state)
				break
			case "gasFeeBump":
				await tuneGasFeeBump(state)
				break
			case "overfill":
				await tuneOverfill(state)
				break
			case "confirmations":
				await tuneConfirmations(state)
				break
			case "rebalancing":
				await tuneRebalancing(state)
				break
			case "vault":
				await tuneVault(state)
				break
			case "allowlist":
				await tuneAllowlist(state)
				break
			case "logging":
				await tuneLogging(state)
				break
		}
	}
}

/** Sections the wizard doesn't re-prompt for survive an update run unchanged. */
function carryPrefillExtras(state: WizardState, prefill?: Prefill): void {
	if (!prefill) return
	const { config } = prefill
	state.maxConcurrentOrders = config.simplex.maxConcurrentOrders ?? state.maxConcurrentOrders
	state.queue = config.simplex.queue ?? state.queue
	state.logging = config.simplex.logging ?? state.logging
	state.gasFeeBump = config.simplex.gasFeeBump
	state.overfillProtection = config.simplex.overfillProtection
	state.rebalancing = config.rebalancing
	state.vault = config.vault
	state.allowlist = config.allowlist
}

async function askNumber(message: string, initial: number, validate?: (n: number) => string | undefined) {
	const value = guard(
		await text({
			message,
			initialValue: String(initial),
			validate: (input) => {
				const parsed = Number((input ?? "").trim())
				if (!Number.isFinite(parsed)) return "Enter a number"
				return validate?.(parsed)
			},
		}),
	)
	return Number(value.trim())
}

async function tuneConcurrency(state: WizardState): Promise<void> {
	why(WHY.concurrency)
	state.maxConcurrentOrders = await askNumber("Max concurrent orders", state.maxConcurrentOrders, (n) =>
		n >= 1 ? undefined : "Must be at least 1",
	)
	state.queue.maxRechecks = await askNumber(
		"Max re-checks before dropping a queued order",
		state.queue.maxRechecks,
		(n) => (n >= 0 ? undefined : "Must be >= 0"),
	)
	state.queue.recheckDelayMs = await askNumber("Delay between re-checks (ms)", state.queue.recheckDelayMs, (n) =>
		n >= 1000 ? undefined : "Use at least 1000ms",
	)
}

async function tuneGasFeeBump(state: WizardState): Promise<void> {
	why(WHY.gasFeeBump)
	state.gasFeeBump = {
		maxPriorityFeePerGasBumpPercent: await askNumber(
			"Priority fee bump % (default 8)",
			state.gasFeeBump?.maxPriorityFeePerGasBumpPercent ?? 8,
			(n) => (n >= 0 ? undefined : "Must be >= 0"),
		),
		maxFeePerGasBumpPercent: await askNumber(
			"Max fee bump % (default 10)",
			state.gasFeeBump?.maxFeePerGasBumpPercent ?? 10,
			(n) => (n >= 0 ? undefined : "Must be >= 0"),
		),
	}
}

async function tuneOverfill(state: WizardState): Promise<void> {
	why(WHY.overfill)
	state.overfillProtection = {
		maxOverfillBps: await askNumber(
			"Max overfill (bps above user-requested output, default 500)",
			state.overfillProtection?.maxOverfillBps ?? 500,
			(n) => (n >= 0 && n <= 10_000 ? undefined : "Between 0 and 10000"),
		),
		maxConsecutiveClamps: await askNumber(
			"Consecutive clamped orders before halting (default 3)",
			state.overfillProtection?.maxConsecutiveClamps ?? 3,
			(n) => (n >= 1 ? undefined : "Must be at least 1"),
		),
	}
}

async function tuneConfirmations(state: WizardState): Promise<void> {
	why(WHY.confirmations)
	for (const strategy of state.strategies) {
		for (const chain of state.chains) {
			const chainId = String(chain.meta.chainId)
			const current = strategy.confirmationPolicies?.[chainId] ?? DEFAULT_CONFIRMATION_POLICIES[chainId]
			note(
				current
					? current.points.map((p) => `$${p.amount} -> ${p.value} confirmations`).join("\n")
					: "No built-in default for this chain — one should be set.",
				`${chain.meta.label} (${strategy.type})`,
			)
			const customize = guard(
				await confirm({ message: `Customize confirmations for ${chain.meta.label}?`, initialValue: !current }),
			)
			if (!customize) continue

			const points: ChainConfirmationPolicy["points"] = []
			while (points.length < 2) {
				const input = guard(
					await text({
						message: "Point as `orderUsd,confirmations` (e.g. `1000,2`); empty line to finish",
						defaultValue: "",
						validate: (value) => {
							const trimmed = (value ?? "").trim()
							if (!trimmed) return undefined
							return parsePointInput(trimmed) ? undefined : "Expected e.g. `1000,2`"
						},
					}),
				)
				const trimmed = (input ?? "").trim()
				if (!trimmed) {
					if (points.length >= 2) break
					log.error("At least 2 points required.")
					continue
				}
				const pair = parsePointInput(trimmed)!
				points.push({ amount: pair.first, value: Number(pair.second) })
			}
			strategy.confirmationPolicies = { ...(strategy.confirmationPolicies ?? {}), [chainId]: { points } }
		}
	}
}

async function tuneRebalancing(state: WizardState): Promise<void> {
	why(WHY.rebalancing)
	const triggerPercentage = await askNumber(
		"Trigger fraction (0.5 = rebalance when a chain drops to 50% of its base)",
		state.rebalancing?.triggerPercentage ?? 0.5,
		(n) => (n > 0 && n < 1 ? undefined : "Between 0 and 1 (exclusive)"),
	)
	const baseBalances: { USDC?: Record<string, string>; USDT?: Record<string, string> } = {}
	for (const symbol of ["USDC", "USDT"] as const) {
		const include = guard(
			await confirm({ message: `Set base balances for ${symbol}?`, initialValue: symbol === "USDC" }),
		)
		if (!include) continue
		const perChain: Record<string, string> = {}
		for (const chain of state.chains) {
			const amount = guard(
				await text({
					message: `${symbol} base balance on ${chain.meta.label} (USD, empty to skip)`,
					initialValue: state.rebalancing?.baseBalances[symbol]?.[String(chain.meta.chainId)] ?? "10000",
					defaultValue: "",
					validate: (value) => {
						const trimmed = (value ?? "").trim()
						if (!trimmed) return undefined
						return Number(trimmed) > 0 ? undefined : "Enter a positive number or leave empty"
					},
				}),
			)
			if ((amount ?? "").trim()) perChain[String(chain.meta.chainId)] = amount.trim()
		}
		if (Object.keys(perChain).length > 0) baseBalances[symbol] = perChain
	}
	if (Object.keys(baseBalances).length === 0) {
		log.warn("No base balances set — skipping rebalancing.")
		state.rebalancing = undefined
		return
	}
	state.rebalancing = { triggerPercentage, baseBalances }
}

async function tuneVault(state: WizardState): Promise<void> {
	why(WHY.vault)
	const vaults: VaultToml[] = [...(state.vault?.vaults ?? [])]
	if (vaults.length > 0) {
		log.info(`Keeping ${vaults.length} existing vault${vaults.length > 1 ? "s" : ""}.`)
	}
	do {
		const chain = guard(
			await select({
				message: "Chain the ERC-4626 vault is on",
				options: state.chains.map((c) => ({ value: c.meta.stateMachineId, label: c.meta.label })),
			}),
		)
		const vault = guard(
			await text({
				message: "Vault address (any ERC-4626, e.g. Aave stataUSDC)",
				validate: (value) => (isAddress((value ?? "").trim()) ? undefined : "Enter a valid EVM address"),
			}),
		)
		const sweep = guard(
			await confirm({
				message: "Sweep idle wallet balance into the vault? (otherwise it's withdraw-only)",
				initialValue: true,
			}),
		)
		const entry: VaultToml = { chain, vault: vault.trim() as HexString }
		if (sweep) {
			const threshold = await askNumber("Sweep when wallet balance reaches (USD)", 5000, (n) =>
				n > 0 ? undefined : "Must be positive",
			)
			const minBalance = await askNumber(
				"Sweep down to (USD; cover fill float + gas spend)",
				Math.min(3000, threshold - 1),
				(n) => (n > 0 && n < threshold ? undefined : `Must be positive and below ${threshold}`),
			)
			entry.threshold = String(threshold)
			entry.minBalance = String(minBalance)
		}
		vaults.push(entry)
	} while (guard(await confirm({ message: "Add another vault?", initialValue: false })))

	state.vault = { ...(state.vault ?? {}), vaults }
}

async function tuneAllowlist(state: WizardState): Promise<void> {
	why(WHY.allowlist)
	const users: string[] = [...(state.allowlist?.users ?? [])]
	for (;;) {
		const address = guard(
			await text({
				message: "Allowed user address (empty line to finish)",
				defaultValue: "",
				validate: (value) => {
					const trimmed = (value ?? "").trim()
					if (!trimmed) return undefined
					return isAddress(trimmed) ? undefined : "Enter a valid EVM address"
				},
			}),
		)
		const trimmed = (address ?? "").trim()
		if (!trimmed) break
		users.push(trimmed)
	}
	if (users.length === 0) {
		log.warn("Empty allowlist would reject every order — leaving the allowlist off.")
		state.allowlist = undefined
		return
	}
	state.allowlist = { ...(state.allowlist ?? {}), users }
}

async function tuneLogging(state: WizardState): Promise<void> {
	why(WHY.logging)
	state.logging = guard(
		await select({
			message: "Log level",
			initialValue: state.logging ?? "info",
			options: ["trace", "debug", "info", "warn", "error"].map((level) => ({ value: level, label: level })),
		}),
	)
}
