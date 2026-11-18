import { formatUnits, parseUnits } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { normalizeSymbol, type AssetRegistry } from "@/config/asset-registry"
import {
	DEFAULT_REBALANCE_INTERVAL_MS,
	validateRebalancingConfig,
	type RebalancingConfig,
	type RebalancingHubToml,
} from "@/config/filler-toml"
import { formatChainKey } from "@/config/interpolated-curve"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { type Logger, moduleLogger } from "./Logger"
import {
	BinanceRebalancer,
	CctpRebalancer,
	Usdt0Rebalancer,
	type BinanceCexConfig,
	type RebalanceMethod,
	type RebalanceRoute,
} from "./rebalancers"

/** Why a chain is left out of a hub, whatever its wallet holds. */
export type RebalanceUncoveredReason = "no-asset" | "no-route" | "vault"

/** Why one rebalancing pass left a chain alone. */
export type RebalanceSkipReason = RebalanceUncoveredReason | "below-threshold" | "balance-unavailable"

/** One chain as a hub sees it: the route that reaches the hub, or why there is none. */
export interface RebalanceChainCoverage {
	chain: string
	/** The route that carries the asset to the hub. Absent when the chain is left out. */
	method?: RebalanceMethod
	/** Why the chain is left out. Absent when it is covered. */
	uncovered?: RebalanceUncoveredReason
}

/** Every other chain the solver runs, as one hub sees it. */
export interface RebalanceHubCoverage {
	asset: string
	hub: string
	chains: RebalanceChainCoverage[]
}

/** One asset a route can bridge, with every chain that could be its hub and what each would cover. */
export interface RebalanceAssetOptions {
	asset: string
	hubs: Array<{ chain: string; chains: RebalanceChainCoverage[] }>
}

/** A transfer into a hub. `amount` is in human units. */
export interface RebalanceTransfer {
	asset: string
	source: string
	hub: string
	amount: string
	method: RebalanceMethod
}

export interface RebalanceSkip {
	asset: string
	chain: string
	hub: string
	reason: RebalanceSkipReason
	/** Solver wallet balance of the asset, human units. Only read on a covered chain. */
	walletBalance?: string
	/** The hub's threshold, human units. Present with `walletBalance`. */
	threshold?: string
}

/** What one pass decided: the transfers it started, and why every other chain was left alone. */
export interface RebalancePass {
	started: RebalanceTransfer[]
	skipped: RebalanceSkip[]
}

/** How the transfers of one pass ended. */
export interface RebalanceOutcome {
	/** Delivered to the hub. `amount` is what was sent, decided when the transfer's turn came. */
	moved: Array<RebalanceTransfer & { txHash?: string }>
	failed: Array<RebalanceTransfer & { error: string }>
	/** No longer due when their turn came: the balance was spent, or the hub was edited. */
	notSent: Array<RebalanceTransfer & { reason: string }>
}

type TransferResult =
	| { moved: RebalanceOutcome["moved"][number] }
	| { failed: RebalanceOutcome["failed"][number] }
	| { notSent: RebalanceOutcome["notSent"][number] }

/** Thrown when a pass is asked for while the previous one is still moving funds. */
export class RebalanceInProgressError extends Error {
	constructor() {
		super("A rebalance is already running")
		this.name = "RebalanceInProgressError"
	}
}

export interface RebalancingServiceOptions {
	/** Enables the Binance route, for pairs no native route connects. */
	binance?: BinanceCexConfig
	/** Whether a vault on `chain` holds `token`. Such a chain is left to its vault sweep. */
	hasVault?: (chain: string, token: HexString) => boolean
}

const sameAddress = (a: string | undefined | null, b: string | undefined | null): boolean =>
	!!a && !!b && a.toLowerCase() === b.toLowerCase()

/**
 * Gathers each configured asset on its hub chain. Once any other chain's wallet holds the
 * hub's `threshold`, everything above `minBalance` is bridged to the hub. Every chain is
 * judged on its own: see `rebalancers/Rules.md`.
 */
export class RebalancingService {
	private readonly logger: Logger
	private readonly walletAddress: HexString
	/** In order of preference: a native route first, Binance where no native route runs. */
	private readonly routes: RebalanceRoute[]

	constructor(
		private readonly chainClientManager: ChainClientManager,
		private readonly configService: FillerConfigService,
		private readonly assetRegistry: AssetRegistry,
		private readonly options: RebalancingServiceOptions = {},
	) {
		this.logger = moduleLogger(configService.loggers, "RebalancingService")
		this.walletAddress = chainClientManager.getSigner().address as HexString
		this.routes = [
			new CctpRebalancer(chainClientManager, configService),
			new Usdt0Rebalancer(chainClientManager, configService),
			...(options.binance ? [new BinanceRebalancer(chainClientManager, configService, options.binance)] : []),
		]
	}

	/** How long to wait between passes, read from the live config. */
	intervalMs(): number {
		return this.configService.getRebalancingConfig()?.intervalMs ?? DEFAULT_REBALANCE_INTERVAL_MS
	}

	/** Every asset some route can bridge. */
	assets(): string[] {
		return [...new Set(this.routes.flatMap((route) => route.assets))]
	}

	/**
	 * Every other chain the solver runs, as a hub for `asset` on `hub` would see it. A chain is
	 * covered when the asset registry names the asset there and one route moves that same
	 * token on both the chain and the hub. A chain with a vault for the asset is left out, so
	 * the vault sweep and the rebalance never compete for one wallet balance.
	 */
	coverage({ asset, chain: hub }: Pick<RebalancingHubToml, "asset" | "chain">): RebalanceChainCoverage[] {
		const symbol = normalizeSymbol(asset)
		const hubToken = this.assetRegistry.getAddress(symbol, hub)
		return this.configService
			.getConfiguredChainIds()
			.map(formatChainKey)
			.filter((chain) => chain !== hub)
			.map((chain): RebalanceChainCoverage => {
				const token = this.assetRegistry.getAddress(symbol, chain)
				if (!token) return { chain, uncovered: "no-asset" }
				const route = this.routes.find(
					(candidate) =>
						sameAddress(candidate.tokenOn(symbol, chain), token) &&
						sameAddress(candidate.tokenOn(symbol, hub), hubToken),
				)
				if (!route) return { chain, uncovered: "no-route" }
				if (this.options.hasVault?.(chain, token)) return { chain, uncovered: "vault" }
				return { chain, method: route.method }
			})
	}

	/**
	 * What an operator can choose from: every asset a route can bridge, each running chain
	 * that could be its hub (the asset registry names the asset there), and what that hub
	 * would cover. Computed from the routes and the registry; no chain is read.
	 */
	catalog(): RebalanceAssetOptions[] {
		const chains = this.configService.getConfiguredChainIds().map(formatChainKey)
		return this.assets().map((asset) => ({
			asset,
			hubs: chains
				.filter((chain) => this.assetRegistry.getAddress(asset, chain) !== null)
				.map((chain) => ({ chain, chains: this.coverage({ asset, chain }) })),
		}))
	}

	/**
	 * Checks a prospective `[rebalancing]` section against the running solver and reports what
	 * each hub would cover. Throws on a section this solver cannot use: a hub on a chain it does
	 * not run, an asset no route carries, or one the registry does not name on the hub.
	 * Changes nothing.
	 */
	preflight(rebalancing: RebalancingConfig): RebalanceHubCoverage[] {
		validateRebalancingConfig(rebalancing, this.configService.getConfiguredChainIds())
		return (rebalancing.hubs ?? []).map((hub) => {
			const asset = normalizeSymbol(hub.asset)
			if (!this.assets().includes(asset)) {
				throw new Error(
					`rebalancing.hubs: no bridge route carries ${asset}; routes exist for ${this.assets().join(" and ")}`,
				)
			}
			if (!this.assetRegistry.getAddress(asset, hub.chain)) {
				throw new Error(`rebalancing.hubs: ${asset} is not in the asset registry on ${hub.chain}`)
			}
			return { asset, hub: hub.chain, chains: this.coverage(hub) }
		})
	}

	/**
	 * Reads every covered chain's wallet balance and decides what to move. Reads only: the
	 * transfers it returns are sent by {@link execute}.
	 */
	async plan(): Promise<RebalancePass> {
		const hubs = this.configService.getRebalancingConfig()?.hubs ?? []
		const decisions = await Promise.all(
			hubs.flatMap((hub) => this.coverage(hub).map((entry) => this.decide(hub, entry))),
		)
		const pass: RebalancePass = { started: [], skipped: [] }
		for (const decision of decisions) {
			if ("reason" in decision) pass.skipped.push(decision)
			else pass.started.push(decision)
		}
		for (const transfer of pass.started) this.logger.info(transfer, "Wallet reached its rebalancing threshold")
		return pass
	}

	private async decide(
		hub: RebalancingHubToml,
		entry: RebalanceChainCoverage,
	): Promise<RebalanceTransfer | RebalanceSkip> {
		const asset = normalizeSymbol(hub.asset)
		const where = { asset, chain: entry.chain, hub: hub.chain }
		if (!entry.method) return { ...where, reason: entry.uncovered ?? "no-route" }

		let wallet: { balance: bigint; decimals: number }
		try {
			wallet = await this.readWallet(asset, entry.chain)
		} catch (error) {
			this.logger.warn({ error, ...where }, "Could not read the wallet balance; chain skipped this pass")
			return { ...where, reason: "balance-unavailable" }
		}

		// Amounts are configured in human units and scaled with this chain's own decimals:
		// one asset has 6 on most chains and 18 on BNB Chain.
		const threshold = parseUnits(hub.threshold, wallet.decimals)
		// Whole cents, rounded down: every route takes the amount as a decimal string, and an
		// 18-decimal remainder is finer than Binance accepts on a withdrawal.
		const cent = 10n ** BigInt(Math.max(wallet.decimals - 2, 0))
		const excess = wallet.balance - parseUnits(hub.minBalance, wallet.decimals)
		const amount = excess - (excess % cent)
		if (wallet.balance < threshold || amount <= 0n) {
			return {
				...where,
				reason: "below-threshold",
				walletBalance: formatUnits(wallet.balance, wallet.decimals),
				threshold: hub.threshold,
			}
		}

		return {
			asset,
			source: entry.chain,
			hub: hub.chain,
			amount: formatUnits(amount, wallet.decimals),
			method: entry.method,
		}
	}

	/**
	 * Sends the transfers of one pass and reports how each ended. Never rejects: a transfer
	 * that fails is reported, and the next pass tries again if the balance is still there.
	 *
	 * Every transfer is sent from the one solver wallet with no nonce manager, so two that send
	 * on the same chain at once would race for its nonce. Each chain is a lane: a transfer waits
	 * for the previous one on every chain it sends on, and transfers that share no chain run in
	 * parallel. The result keeps plan order.
	 *
	 * `cancelled` is asked before each transfer is sent. Once it answers true, the transfers
	 * still waiting their turn are not sent; one already under way runs to its end.
	 */
	async execute(transfers: RebalanceTransfer[], cancelled: () => boolean = () => false): Promise<RebalanceOutcome> {
		const results: TransferResult[] = new Array(transfers.length)
		const lanes = new Map<string, Promise<void>>()
		transfers.forEach((transfer, index) => {
			const route = this.routes.find((candidate) => candidate.method === transfer.method)
			const chains = route?.sendsOnDestination ? [transfer.source, transfer.hub] : [transfer.source]
			const previous = chains.map((chain) => lanes.get(chain) ?? Promise.resolve())
			const run = Promise.all(previous).then(async () => {
				results[index] = cancelled()
					? { notSent: { ...transfer, reason: "cancelled" } }
					: await this.send(route, transfer)
			})
			for (const chain of chains) lanes.set(chain, run)
		})
		await Promise.all(lanes.values())

		const outcome: RebalanceOutcome = { moved: [], failed: [], notSent: [] }
		for (const result of results) {
			if ("moved" in result) outcome.moved.push(result.moved)
			else if ("failed" in result) outcome.failed.push(result.failed)
			else outcome.notSent.push(result.notSent)
		}
		return outcome
	}

	private async send(route: RebalanceRoute | undefined, planned: RebalanceTransfer): Promise<TransferResult> {
		let transfer = planned
		try {
			if (!route) throw new Error(`The ${planned.method} route is not available`)

			// A transfer can wait in its lane for many minutes behind another, while fills keep
			// spending the wallet. So it is decided again now, from the balance and the hub as
			// they are now: a stale amount would take the chain below its minimum balance.
			const current = await this.decideAgain(planned)
			if ("reason" in current) {
				this.logger.info({ ...planned, reason: current.reason }, "Rebalancing transfer no longer due; not sent")
				return { notSent: { ...planned, reason: current.reason } }
			}
			transfer = current

			this.logger.info(transfer, "Sending rebalancing transfer")
			const receipt = await route.send({
				asset: transfer.asset,
				amount: transfer.amount,
				source: transfer.source,
				destination: transfer.hub,
			})
			this.logger.info({ ...transfer, ...receipt }, "Rebalancing transfer delivered")
			return { moved: { ...transfer, ...(receipt.txHash ? { txHash: receipt.txHash } : {}) } }
		} catch (error) {
			this.logger.error({ error, ...transfer }, "Rebalancing transfer failed")
			return { failed: { ...transfer, error: error instanceof Error ? error.message : String(error) } }
		}
	}

	/**
	 * The planned transfer as it stands now, or why it is no longer due. The hub is looked up
	 * in the live config, so switching rebalancing off or moving a hub also stops the
	 * transfers still waiting their turn.
	 */
	private async decideAgain(planned: RebalanceTransfer): Promise<RebalanceTransfer | { reason: string }> {
		const hub = (this.configService.getRebalancingConfig()?.hubs ?? []).find(
			(candidate) => normalizeSymbol(candidate.asset) === planned.asset && candidate.chain === planned.hub,
		)
		if (!hub) return { reason: "hub-changed" }
		const entry = this.coverage(hub).find((candidate) => candidate.chain === planned.source)
		// A different route would need a different lane; the next pass plans it afresh.
		if (!entry || (entry.method && entry.method !== planned.method)) return { reason: "hub-changed" }
		return this.decide(hub, entry)
	}

	/** The solver wallet's balance of `asset` on `chain`, with the token's own decimals. */
	private async readWallet(asset: string, chain: string): Promise<{ balance: bigint; decimals: number }> {
		const token = this.assetRegistry.getAddress(asset, chain)
		if (!token) throw new Error(`${asset} is not in the asset registry on ${chain}`)
		const publicClient = this.chainClientManager.getPublicClient(chain)
		const [decimals, balance] = await Promise.all([
			publicClient.readContract({ address: token, abi: ERC20_ABI, functionName: "decimals" }),
			publicClient.readContract({
				address: token,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [this.walletAddress],
			}),
		])
		return { balance: balance as bigint, decimals: Number(decimals) }
	}
}
