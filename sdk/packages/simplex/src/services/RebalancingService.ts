import { formatUnits } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { type HexString, parseStateMachineId } from "@hyperbridge/sdk"
import { Decimal } from "decimal.js"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import { ERC20_ABI } from "@/config/abis/ERC20"
import {
	BinanceRebalancer,
	CctpRebalancer,
	Usdt0Rebalancer,
	type BinanceCexConfig,
	type CexRebalanceEstimate,
	type CexRebalanceResult,
	type RebalanceMethod,
	type RouteDecision,
	type RebalanceOptions,
	type UnifiedRebalanceOptions,
	type Usdt0EstimateResult,
	type Usdt0TransferResult,
} from "./rebalancers"

/**
 * Represents a planned transfer from one chain to another for portfolio-level rebalancing.
 */
export interface PlannedTransfer {
	sourceChain: string
	destChain: string
	coin: "USDC" | "USDT"
	amount: string // Human-readable amount (e.g., "5000.00")
}

/**
 * Result of checking portfolio rebalancing triggers.
 */
export interface TriggerCheckResult {
	triggered: boolean
	triggeredChains: Array<{
		chain: string
		asset: "USDC" | "USDT"
		deficit: number
	}>
}

/**
 * Result of a portfolio rebalancing operation.
 */
export interface RebalancingResult {
	success: boolean
	transfers: PlannedTransfer[]
	executedTransfers: Array<{
		transfer: PlannedTransfer
		result: unknown
	}>
	errors?: string[]
}

// Chain IDs where CCTP is supported (Circle native mint/burn)
const CCTP_CHAINS = new Set([1, 42161, 8453, 137, 130])

// Chain IDs where USDT0 (LayerZero OFT) is supported
const USDT0_CHAINS = new Set([1, 42161, 8453, 137])

// Chain IDs where Binance CEX route is available
const BINANCE_CHAINS = new Set([1, 56, 137, 42161, 8453])

function selectRoute(coin: "USDC" | "USDT", sourceChainId: number, destChainId: number): RouteDecision {
	// USDC: prefer CCTP if both chains support it
	if (coin === "USDC") {
		if (CCTP_CHAINS.has(sourceChainId) && CCTP_CHAINS.has(destChainId)) {
			return { method: "cctp", reason: "Both chains support CCTP for USDC" }
		}
	}

	// USDT: prefer USDT0/OFT if both chains support it
	if (coin === "USDT") {
		if (USDT0_CHAINS.has(sourceChainId) && USDT0_CHAINS.has(destChainId)) {
			return { method: "usdt0", reason: "Both chains support USDT0 OFT" }
		}
	}

	// Fallback to CEX
	if (BINANCE_CHAINS.has(sourceChainId) && BINANCE_CHAINS.has(destChainId)) {
		return {
			method: "cex",
			reason: `${coin} not natively bridgeable between chainId ${sourceChainId} → ${destChainId}, using Binance`,
		}
	}

	throw new Error(`No rebalancing route available for ${coin} from chainId ${sourceChainId} to ${destChainId}`)
}

/**
 * RebalancingService - Facade over CCTP, USDT0, and Binance CEX rebalancers.
 * Provides both low-level methods (sendCctp, sendUsdt0, sendViaCex) and a high-level router (rebalance).
 */
export class RebalancingService {
	private readonly chainClientManager: ChainClientManager
	private readonly configService: FillerConfigService
	private readonly walletAddress: string

	private readonly cctpRebalancer: CctpRebalancer
	private readonly usdt0Rebalancer: Usdt0Rebalancer
	private readonly binanceRebalancer?: BinanceRebalancer

	private readonly logger = getLogger("RebalancingService")

	constructor(
		chainClientManager: ChainClientManager,
		configService: FillerConfigService,
		privateKey: HexString,
		binanceConfig?: BinanceCexConfig,
	) {
		this.chainClientManager = chainClientManager
		this.configService = configService

		const account = privateKeyToAccount(privateKey as `0x${string}`)
		this.walletAddress = account.address

		this.cctpRebalancer = new CctpRebalancer(chainClientManager, configService, privateKey)
		this.usdt0Rebalancer = new Usdt0Rebalancer(chainClientManager, configService, privateKey)
		this.binanceRebalancer = binanceConfig
			? new BinanceRebalancer(chainClientManager, configService, privateKey, binanceConfig)
			: undefined
	}

	// ------------------------------------------------------------------------
	// CCTP wrappers (backwards compatible)
	// ------------------------------------------------------------------------

	async sendCctp(options: RebalanceOptions) {
		return this.cctpRebalancer.sendCctp(options)
	}

	async estimateCctp(options: RebalanceOptions) {
		return this.cctpRebalancer.estimateCctp(options)
	}

	async retrySendCctp(failedResult: Awaited<ReturnType<CctpRebalancer["sendCctp"]>>) {
		return this.cctpRebalancer.retrySendCctp(failedResult)
	}

	// ------------------------------------------------------------------------
	// USDT0 wrappers (backwards compatible)
	// ------------------------------------------------------------------------

	async sendUsdt0(options: RebalanceOptions): Promise<Usdt0TransferResult> {
		return this.usdt0Rebalancer.sendUsdt0(options)
	}

	async estimateUsdt0(options: RebalanceOptions): Promise<Usdt0EstimateResult> {
		return this.usdt0Rebalancer.estimateUsdt0(options)
	}

	// ------------------------------------------------------------------------
	// CEX wrappers
	// ------------------------------------------------------------------------

	async sendViaCex(options: UnifiedRebalanceOptions): Promise<CexRebalanceResult> {
		if (!this.binanceRebalancer) {
			throw new Error("Binance CEX rebalancer not configured")
		}
		return this.binanceRebalancer.sendViaCex(options)
	}

	async estimateCexRebalance(options: UnifiedRebalanceOptions): Promise<CexRebalanceEstimate> {
		if (!this.binanceRebalancer) {
			throw new Error("Binance CEX rebalancer not configured")
		}
		return this.binanceRebalancer.estimateCexRebalance(options)
	}

	// ------------------------------------------------------------------------
	// High-level router
	// ------------------------------------------------------------------------

	/**
	 * Execute a single cross-chain transfer, selecting the best route (CCTP / USDT0 / CEX).
	 * Requires coin to be specified for routing decisions.
	 */
	async rebalance(
		options: UnifiedRebalanceOptions,
	): Promise<Awaited<ReturnType<CctpRebalancer["sendCctp"]>> | Usdt0TransferResult | CexRebalanceResult> {
		const { amount, coin, source, destination } = options

		const sourceChainId = parseStateMachineId(source).stateId.Evm!
		const destChainId = parseStateMachineId(destination).stateId.Evm!

		const route = selectRoute(coin, sourceChainId, destChainId)
		this.logger.info({ ...route, coin, source, destination, amount }, "Route selected")

		switch (route.method) {
			case "cctp":
				return this.sendCctp({ amount, source, destination })

			case "usdt0":
				return this.sendUsdt0({ amount, source, destination })

			case "cex":
				return this.sendViaCex(options)
		}
	}

	// ------------------------------------------------------------------------
	// Portfolio-level rebalancing (multi-chain)
	// ------------------------------------------------------------------------

	/**
	 * Check if any chains/assets have triggered portfolio rebalancing thresholds.
	 */
	async checkRebalanceTriggers(): Promise<TriggerCheckResult> {
		const rebalancingConfig = this.configService.getRebalancingConfig()
		if (!rebalancingConfig) {
			this.logger.debug("Rebalancing config not found, skipping trigger check")
			return { triggered: false, triggeredChains: [] }
		}

		const triggerPercentage = rebalancingConfig.triggerPercentage
		const chainIds = this.configService.getConfiguredChainIds()
		const triggeredChains: TriggerCheckResult["triggeredChains"] = []

		for (const chainId of chainIds) {
			const chain = `EVM-${chainId}`

			for (const asset of ["USDC", "USDT"] as const) {
				const baseBalance = this.configService.getBaseBalance(chainId, asset)
				if (baseBalance === undefined) {
					continue // Skip if base balance not configured
				}

				const currentBalance = await this.getCurrentBalance(chain, asset)
				if (currentBalance === undefined) {
					this.logger.warn({ chain, asset }, "Could not fetch current balance")
					continue
				}

				const threshold = baseBalance * (1 - triggerPercentage)
				if (currentBalance <= threshold) {
					const deficit = baseBalance - currentBalance
					triggeredChains.push({
						chain,
						asset,
						deficit,
					})
					this.logger.info(
						{ chain, asset, currentBalance, baseBalance, threshold, deficit },
						"Portfolio rebalancing trigger detected",
					)
				}
			}
		}

		return {
			triggered: triggeredChains.length > 0,
			triggeredChains,
		}
	}

	/**
	 * Plan portfolio-level rebalancing transfers for triggered chains using greedy matching.
	 */
	async planPortfolioRebalancing(triggeredChains: TriggerCheckResult["triggeredChains"]): Promise<PlannedTransfer[]> {
		const transfers: PlannedTransfer[] = []
		const chainIds = this.configService.getConfiguredChainIds()

		// Group by asset
		const deficitsByAsset = new Map<"USDC" | "USDT", Array<{ chain: string; deficit: number }>>()
		for (const triggered of triggeredChains) {
			if (!deficitsByAsset.has(triggered.asset)) {
				deficitsByAsset.set(triggered.asset, [])
			}
			deficitsByAsset.get(triggered.asset)!.push({
				chain: triggered.chain,
				deficit: triggered.deficit,
			})
		}

		// Plan transfers for each asset separately
		for (const [asset, deficits] of deficitsByAsset.entries()) {
			const assetTransfers = await this.planTransfersForAsset(asset, deficits, chainIds)
			transfers.push(...assetTransfers)
		}

		return transfers
	}

	/**
	 * Plan transfers for a specific asset using greedy surplus–deficit matching.
	 */
	private async planTransfersForAsset(
		asset: "USDC" | "USDT",
		deficits: Array<{ chain: string; deficit: number }>,
		allChainIds: number[],
	): Promise<PlannedTransfer[]> {
		const transfers: PlannedTransfer[] = []

		// Compute surpluses for this asset
		const surpluses: Array<{ chain: string; surplus: number }> = []
		for (const chainId of allChainIds) {
			const chain = `EVM-${chainId}`
			const baseBalance = this.configService.getBaseBalance(chainId, asset)
			if (baseBalance === undefined) {
				continue
			}

			const currentBalance = await this.getCurrentBalance(chain, asset)
			if (currentBalance === undefined) {
				continue
			}

			if (currentBalance > baseBalance) {
				const surplus = currentBalance - baseBalance
				surpluses.push({ chain, surplus })
			}
		}

		// Check if total surplus is sufficient
		const totalDeficit = deficits.reduce((sum, d) => sum + d.deficit, 0)
		const totalSurplus = surpluses.reduce((sum, s) => sum + s.surplus, 0)

		if (totalSurplus < totalDeficit) {
			throw new Error(
				`Insufficient surplus to bring ${asset} balances back to base. ` +
					`Total deficit: ${totalDeficit}, Total surplus: ${totalSurplus}`,
			)
		}

		// Greedy matching: sort deficits and surpluses in descending order
		const sortedDeficits = [...deficits].sort((a, b) => b.deficit - a.deficit)
		// Create mutable copies of surpluses for tracking remaining amounts
		const workingSurpluses = surpluses.map((s) => ({ ...s })).sort((a, b) => b.surplus - a.surplus)

		// Match deficits with surpluses
		for (const deficitInfo of sortedDeficits) {
			let remainingDeficit = deficitInfo.deficit

			for (const surplusInfo of workingSurpluses) {
				if (remainingDeficit <= 0) {
					break
				}

				if (surplusInfo.surplus <= 0) {
					continue
				}

				// Skip if source and dest are the same chain
				if (surplusInfo.chain === deficitInfo.chain) {
					continue
				}

				const transferAmount = Math.min(remainingDeficit, surplusInfo.surplus)
				transfers.push({
					sourceChain: surplusInfo.chain,
					destChain: deficitInfo.chain,
					coin: asset,
					amount: transferAmount.toFixed(2),
				})

				remainingDeficit -= transferAmount
				surplusInfo.surplus -= transferAmount
			}

			if (remainingDeficit > 0) {
				throw new Error(
					`Could not fully satisfy deficit for ${asset} on ${deficitInfo.chain}. ` +
						`Remaining deficit: ${remainingDeficit}`,
				)
			}
		}

		return transfers
	}

	/**
	 * Execute planned portfolio-level transfers via this service.
	 */
	async executePortfolioRebalancing(transfers: PlannedTransfer[]): Promise<RebalancingResult> {
		const executedTransfers: RebalancingResult["executedTransfers"] = []
		const errors: string[] = []

		this.logger.info({ transferCount: transfers.length }, "Executing portfolio rebalancing transfers")

		// Execute transfers in parallel
		const transferPromises = transfers.map(async (transfer) => {
			try {
				this.logger.info(
					{
						sourceChain: transfer.sourceChain,
						destChain: transfer.destChain,
						coin: transfer.coin,
						amount: transfer.amount,
					},
					"Executing portfolio transfer",
				)

				const result = await this.rebalance({
					amount: transfer.amount,
					coin: transfer.coin,
					source: transfer.sourceChain,
					destination: transfer.destChain,
				})

				return { transfer, result }
			} catch (error) {
				const errorMsg = `Portfolio transfer failed: ${transfer.sourceChain} -> ${transfer.destChain} (${transfer.coin} ${transfer.amount}): ${
					error instanceof Error ? error.message : String(error)
				}`
				this.logger.error({ error, transfer }, errorMsg)
				errors.push(errorMsg)
				return null
			}
		})

		const results = await Promise.all(transferPromises)
		for (const result of results) {
			if (result !== null) {
				executedTransfers.push(result)
			}
		}

		return {
			success: errors.length === 0,
			transfers,
			executedTransfers,
			errors: errors.length > 0 ? errors : undefined,
		}
	}

	/**
	 * High-level portfolio rebalance:
	 * 1) Check triggers
	 * 2) Plan transfers
	 * 3) Execute transfers
	 */
	async rebalancePortfolio(): Promise<RebalancingResult> {
		const triggerCheck = await this.checkRebalanceTriggers()

		if (!triggerCheck.triggered) {
			this.logger.debug("No portfolio rebalancing triggers detected")
			return {
				success: true,
				transfers: [],
				executedTransfers: [],
			}
		}

		this.logger.info(
			{ triggeredChains: triggerCheck.triggeredChains },
			"Portfolio rebalancing triggers detected, planning transfers",
		)

		const plannedTransfers = await this.planPortfolioRebalancing(triggerCheck.triggeredChains)
		this.logger.info({ transferCount: plannedTransfers.length }, "Portfolio transfer plan created")

		if (plannedTransfers.length === 0) {
			return {
				success: true,
				transfers: [],
				executedTransfers: [],
			}
		}

		return await this.executePortfolioRebalancing(plannedTransfers)
	}

	/**
	 * Get current balance for a token on a chain.
	 * Returns balance in normalized USD value (treating token as $1).
	 */
	private async getCurrentBalance(chain: string, asset: "USDC" | "USDT"): Promise<number | undefined> {
		try {
			const publicClient = this.chainClientManager.getPublicClient(chain)
			const tokenAddress =
				asset === "USDC" ? this.configService.getUsdcAsset(chain) : this.configService.getUsdtAsset(chain)

			if (!tokenAddress || tokenAddress === "0x") {
				this.logger.warn({ chain, asset }, "Token address not configured")
				return undefined
			}

			// Get decimals
			const decimals = await publicClient.readContract({
				address: tokenAddress as `0x${string}`,
				abi: ERC20_ABI,
				functionName: "decimals",
			})

			// Get balance
			const balanceWei = await publicClient.readContract({
				address: tokenAddress as `0x${string}`,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [this.walletAddress as `0x${string}`],
			})

			// Convert to normalized USD value
			const balanceDecimal = new Decimal(formatUnits(balanceWei, Number(decimals)))
			return balanceDecimal.toNumber()
		} catch (error) {
			this.logger.error({ error, chain, asset }, "Failed to get current balance")
			return undefined
		}
	}

	async estimate(
		options: UnifiedRebalanceOptions,
	): Promise<
		| { method: "cctp"; estimate: Awaited<ReturnType<CctpRebalancer["estimateCctp"]>> }
		| { method: "usdt0"; estimate: Usdt0EstimateResult }
		| { method: "cex"; estimate: CexRebalanceEstimate }
	> {
		const { coin, source, destination, amount } = options

		const sourceChainId = parseStateMachineId(source).stateId.Evm!
		const destChainId = parseStateMachineId(destination).stateId.Evm!

		const route = selectRoute(coin, sourceChainId, destChainId)

		switch (route.method) {
			case "cctp":
				return {
					method: "cctp",
					estimate: await this.estimateCctp({ amount, source, destination }),
				}

			case "usdt0":
				return {
					method: "usdt0",
					estimate: await this.estimateUsdt0({ amount, source, destination }),
				}

			case "cex":
				return {
					method: "cex",
					estimate: await this.estimateCexRebalance(options),
				}
		}
	}
}
