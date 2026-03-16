import { EventMonitor } from "./event-monitor"
import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	FillerConfig,
	ChainConfig,
	getChainId,
	retryPromise,
	type HexString,
	IntentsCoprocessor,
} from "@hyperbridge/sdk"
import pQueue from "p-queue"
import { privateKeyToAddress } from "viem/accounts"
import {
	BidStorageService,
	ChainClientManager,
	ContractInteractionService,
	DelegationService,
	RebalancingService,
} from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import { Decimal } from "decimal.js"

export class IntentFiller {
	public monitor: EventMonitor
	private strategies: FillerStrategy[]
	private chainQueues: Map<number, pQueue>
	private globalQueue: pQueue
	private chainClientManager: ChainClientManager
	private contractService: ContractInteractionService
	private delegationService?: DelegationService
	private rebalancingService?: RebalancingService
	private bidStorage?: BidStorageService
	private retractionQueue: pQueue
	private pendingRetractions = new Set<string>()
	private rebalancingInterval?: NodeJS.Timeout
	private retractionSweepInterval?: NodeJS.Timeout
	private hyperbridge: Promise<IntentsCoprocessor> | undefined = undefined
	private config: FillerConfig
	private configService: FillerConfigService
	private privateKey: HexString
	private logger = getLogger("intent-filler")

	constructor(
		chainConfigs: ChainConfig[],
		strategies: FillerStrategy[],
		config: FillerConfig,
		configService: FillerConfigService,
		chainClientManager: ChainClientManager,
		contractService: ContractInteractionService,
		privateKey: HexString,
		rebalancingService?: RebalancingService,
		bidStorage?: BidStorageService,
	) {
		this.configService = configService
		this.privateKey = privateKey
		this.chainClientManager = chainClientManager
		this.contractService = contractService
		this.rebalancingService = rebalancingService
		this.bidStorage = bidStorage
		const fillerAddress = privateKeyToAddress(privateKey) as HexString
		this.monitor = new EventMonitor(chainConfigs, configService, this.chainClientManager, fillerAddress)
		this.strategies = strategies
		this.config = config

		this.chainQueues = new Map()
		chainConfigs.forEach((chainConfig) => {
			// 1 order per chain at a time due to EVM constraints
			this.chainQueues.set(chainConfig.chainId, new pQueue({ concurrency: 1 }))
		})

		this.globalQueue = new pQueue({
			concurrency: config.maxConcurrentOrders || 5,
		})

		this.retractionQueue = new pQueue({ concurrency: 1 })

		const hyperbridgeWsUrl = configService.getHyperbridgeWsUrl()
		const substrateKey = configService.getSubstratePrivateKey()

		if (hyperbridgeWsUrl && substrateKey) {
			this.hyperbridge = IntentsCoprocessor.connect(hyperbridgeWsUrl, substrateKey)
		}

		// Set up event handlers
		this.monitor.on("newOrder", ({ order, transactionHash }) => {
			this.handleNewOrder(order, transactionHash)
		})

		this.monitor.on("orderFilledOnChain", ({ commitment, filler, chainId }) => {
			this.handleOrderFilledOnChain(commitment as HexString, filler, chainId)
		})
	}

	/**
	 * Initializes the filler, including setting up EIP-7702 delegation if solver selection is active on any chain.
	 * This should be called before start().
	 */
	public async initialize(): Promise<void> {
		// Check which chains have solver selection active
		const chainIds = this.configService.getConfiguredChainIds()
		const chainsWithSolverSelection: string[] = []

		for (const chainId of chainIds) {
			const chain = `EVM-${chainId}`
			const isActive = await this.contractService.isSolverSelectionActive(chain)
			if (isActive) {
				chainsWithSolverSelection.push(chain)
				this.logger.info({ chain }, "Solver selection is active on chain")
			}
		}

		// Set up delegation service on chains where solver selection is active
		if (chainsWithSolverSelection.length > 0 && this.hyperbridge) {
			this.delegationService = new DelegationService(this.chainClientManager, this.configService, this.privateKey)
			this.logger.info(
				{ chains: chainsWithSolverSelection },
				"Setting up EIP-7702 delegation on chains with solver selection",
			)
			const result = await this.delegationService.setupDelegationOnChains(chainsWithSolverSelection)
			if (!result.success) {
				this.logger.warn({ results: result.results }, "Some chains failed EIP-7702 delegation setup")
			}
		}
	}

	public start(): void {
		this.monitor.startListening()

		// Start periodic rebalancing if service is configured
		if (this.rebalancingService) {
			this.startRebalancing()
		}

		if (this.bidStorage && this.hyperbridge) {
			this.startRetractionSweep()
		}
	}

	/**
	 * Start periodic rebalancing checks.
	 * Checks every 5 minutes for triggers and executes rebalancing if needed.
	 */
	private startRebalancing(): void {
		// Run initial check after 30 seconds (to let the filler start up)
		setTimeout(() => {
			this.checkAndRebalance().catch((error) => {
				this.logger.error({ error }, "Error in initial rebalancing check")
			})
		}, 30_000)

		// Then check every 5 minutes
		this.rebalancingInterval = setInterval(
			() => {
				this.checkAndRebalance().catch((error) => {
					this.logger.error({ error }, "Error in periodic rebalancing check")
				})
			},
			5 * 60 * 1000,
		) // 5 minutes

		this.logger.info("Periodic rebalancing checks started (every 5 minutes)")
	}

	/**
	 * Check for rebalancing triggers and execute if needed.
	 */
	private async checkAndRebalance(): Promise<void> {
		if (!this.rebalancingService) {
			return
		}

		try {
			const result = await this.rebalancingService.rebalancePortfolio()
			if (result.success && result.transfers.length > 0) {
				this.logger.info(
					{
						transferCount: result.transfers.length,
						executedCount: result.executedTransfers.length,
					},
					"Portfolio rebalancing completed",
				)
			} else if (result.transfers.length === 0) {
				this.logger.debug("No rebalancing needed")
			}
		} catch (error) {
			this.logger.error({ error }, "Portfolio rebalancing failed")
		}
	}

	private startRetractionSweep(): void {
		const BID_TTL_MS = 60 * 60 * 1000 // 1 hour
		const SWEEP_INTERVAL_MS = 5 * 60 * 1000 // 5 minutes

		this.retractionSweepInterval = setInterval(() => {
			this.sweepExpiredBids(BID_TTL_MS).catch((error) => {
				this.logger.error({ error }, "Error in retraction sweep")
			})
		}, SWEEP_INTERVAL_MS)

		this.logger.info("Periodic retraction sweep started (every 5 minutes, 1h TTL)")
	}

	private async sweepExpiredBids(maxAgeMs: number): Promise<void> {
		if (!this.bidStorage || !this.hyperbridge) {
			return
		}

		const expired = this.bidStorage.getExpiredUnretractedBids(maxAgeMs)
		if (expired.length === 0) {
			return
		}

		this.logger.info({ count: expired.length }, "Sweeping expired unretracted bids")

		for (const bid of expired) {
			this.enqueueRetraction(bid.commitment)
		}
	}

	public async stop(): Promise<void> {
		this.monitor.stopListening()

		// Stop rebalancing interval
		if (this.rebalancingInterval) {
			clearInterval(this.rebalancingInterval)
			this.rebalancingInterval = undefined
			this.logger.info("Periodic rebalancing checks stopped")
		}

		if (this.retractionSweepInterval) {
			clearInterval(this.retractionSweepInterval)
			this.retractionSweepInterval = undefined
			this.logger.info("Periodic retraction sweep stopped")
		}

		// Wait for all queues to complete
		const promises: Promise<void>[] = []
		this.chainQueues.forEach((queue) => {
			promises.push(queue.onIdle())
		})
		promises.push(this.globalQueue.onIdle())
		promises.push(this.retractionQueue.onIdle())

		await Promise.all(promises)

		// Withdraw any remaining EntryPoint deposits back to the solver EOA
		await this.contractService.withdrawAllEntryPointDeposits()

		// Disconnect shared Hyperbridge connection
		if (this.hyperbridge) {
			const service = await this.hyperbridge.catch(() => null)
			await service?.disconnect()
		}

		this.logger.info("All orders processed, filler stopped")
	}

	// Operations

	private handleNewOrder(order: Order, transactionHash: string): void {
		// Use the global queue for the initial analysis
		// This can happen in parallel for PublicClient orders
		this.globalQueue.add(async () => {
			this.logger.info({ orderId: order.id }, "New order detected")
			try {
				// Early check: if solver selection is active, ensure hyperbridge is configured
				const solverSelectionActive = this.contractService.getCache().getSolverSelection(order.destination)
				if (solverSelectionActive == null) {
					this.logger.error({ orderId: order.id }, "Shared cache is not initialized")
					return
				}
				if (solverSelectionActive && !this.hyperbridge) {
					this.logger.error(
						{ orderId: order.id },
						"Solver selection is active but Hyperbridge is not configured. Skipping order.",
					)
					return
				}

				const sourceClient = this.chainClientManager.getPublicClient(order.source)
				// Base layer: stable-only USD value from ContractInteractionService
				const baseInputUsd = await this.contractService.getInputUsdValue(order)

				const canFillCache = new Map<FillerStrategy, boolean>()
				for (const strategy of this.strategies) {
					try {
						canFillCache.set(strategy, await strategy.canFill(order))
					} catch (err) {
						this.logger.error({ orderId: order.id, strategy: strategy.name, err }, "Error checking canFill")
						canFillCache.set(strategy, false)
					}
				}

				let inputUsdValue = baseInputUsd
				for (const [strategy, canFill] of canFillCache) {
					if (!canFill || typeof strategy.getOrderUsdValue !== "function") continue
					try {
						const stratValue = await strategy.getOrderUsdValue(order)

						if (stratValue != null) {
							inputUsdValue = Decimal.max(baseInputUsd, stratValue.inputUsd)
							break
						}
					} catch (err) {
						this.logger.error(
							{ orderId: order.id, strategy: strategy.name, err },
							"Error getting strategy-specific inputUsdValue",
						)
					}
				}

				const isCrossChain = order.source !== order.destination
				let requiredConfirmations = 0
				if (isCrossChain) {
					const hasConfirmationPolicy = [...canFillCache].some(
						([strategy, canFill]) => canFill && strategy.confirmationPolicy,
					)
					if (!hasConfirmationPolicy) {
						this.logger.warn(
							{ orderId: order.id, source: order.source, destination: order.destination },
							"Skipping cross-chain order: no strategy has a confirmation policy configured",
						)
						return
					}
					for (const [strategy, canFill] of canFillCache) {
						if (!canFill || !strategy.confirmationPolicy) continue
						requiredConfirmations = Math.max(
							requiredConfirmations,
							strategy.confirmationPolicy.getConfirmationBlocks(
								getChainId(order.source)!,
								inputUsdValue.toNumber(),
							),
						)
					}
				}

				// Run confirmation waiting and evaluation in parallel.
				// The AbortController lets evaluateOrder cancel the confirmation
				// loop early when the order turns out to be unprofitable.
				const abortController = new AbortController()

				const waitForConfirmations = async (): Promise<void> => {
					let currentConfirmations = await retryPromise(
						() =>
							sourceClient.getTransactionConfirmations({
								hash: transactionHash as HexString,
							}),
						{
							maxRetries: 3,
							backoffMs: 250,
							logMessage: "Failed to get initial transaction confirmations",
						},
					)

					this.logger.info(
						{ orderId: order.id, requiredConfirmations, currentConfirmations },
						"Order confirmation requirements",
					)

					while (currentConfirmations < requiredConfirmations) {
						if (abortController.signal.aborted) return
						await new Promise((resolve) => setTimeout(resolve, 300)) // Wait 300ms
						if (abortController.signal.aborted) return
						currentConfirmations = await retryPromise(
							() =>
								sourceClient.getTransactionConfirmations({
									hash: transactionHash as HexString,
								}),
							{
								maxRetries: 3,
								backoffMs: 250,
								logMessage: "Failed to get transaction confirmations",
							},
						)
						this.logger.debug({ orderId: order.id, currentConfirmations }, "Order confirmation progress")
					}

					this.logger.info({ orderId: order.id, currentConfirmations }, "Order confirmed on source chain")
				}

				// Run confirmation and evaluation in parallel
				const [, evaluationResult] = await Promise.all([
					waitForConfirmations(),
					this.evaluateOrder(order, canFillCache).then((result) => {
						if (!result) abortController.abort()
						return result
					}),
				])

				// Execute immediately
				if (evaluationResult) {
					this.executeOrder(order, evaluationResult.strategy, solverSelectionActive)
				}
			} catch (error) {
				this.logger.error({ orderId: order.id, err: error }, "Error processing order")
			}
		})
	}

	private async evaluateOrder(
		order: Order,
		canFillCache: Map<FillerStrategy, boolean>,
	): Promise<{ strategy: FillerStrategy; profitability: number } | null> {
		// Check if watch-only mode is enabled for the destination chain
		const destChainId = getChainId(order.destination)
		const isWatchOnly =
			destChainId !== undefined &&
			this.config.watchOnly !== undefined &&
			typeof this.config.watchOnly === "object" &&
			this.config.watchOnly[destChainId] === true

		if (isWatchOnly) {
			this.logger.info(
				{
					orderId: order.id,
					sourceChain: order.source,
					destChain: order.destination,
					destChainId,
					user: order.user,
					inputs: order.inputs,
					outputs: order.output.assets,
					watchOnly: true,
				},
				"Order detected in watch-only mode (execution skipped)",
			)
			this.monitor.emit("orderDetected", { orderId: order.id, order, watchOnly: true })
			return null
		}

		const eligibleStrategies = await Promise.all(
			this.strategies.map(async (strategy) => {
				if (!canFillCache.get(strategy)) return null

				const profitability = await strategy.calculateProfitability(order)
				return { strategy, profitability }
			}),
		)

		const validStrategies = eligibleStrategies
			.filter((s): s is NonNullable<typeof s> => s !== null && s.profitability > 0)
			.sort((a, b) => b.profitability - a.profitability)

		if (validStrategies.length === 0) {
			this.logger.warn({ orderId: order.id }, "No profitable strategy found for order")
			return null
		}

		this.logger.info(
			{
				orderId: order.id,
				strategy: validStrategies[0].strategy.name,
				profitability: validStrategies[0].profitability.toString(),
			},
			"Order evaluation complete - profitable strategy found",
		)

		return validStrategies[0]
	}

	private executeOrder(order: Order, bestStrategy: FillerStrategy, solverSelectionActive: boolean): void {
		// Get the chain-specific queue
		const chainQueue = this.chainQueues.get(getChainId(order.destination)!)
		if (!chainQueue) {
			this.logger.error({ chain: order.destination }, "No queue configured for chain")
			return
		}

		// Execute with the most profitable strategy using the chain-specific queue
		// This ensures transactions for the same chain are processed sequentially
		chainQueue.add(async () => {
			this.logger.info(
				{ orderId: order.id, strategy: bestStrategy.name, chain: order.destination },
				"Executing order",
			)

			try {
				if (solverSelectionActive) {
					this.contractService.ensureEntryPointDeposit(order).catch((err) => {
						this.logger.error({ orderId: order.id, err }, "Background EntryPoint deposit top-up failed")
					})
				}

				const hyperbridgeService = solverSelectionActive ? await this.hyperbridge : undefined
				const result = await bestStrategy.executeOrder(order, hyperbridgeService)
				this.logger.info({ orderId: order.id, result }, "Order execution completed")
				if (result.success) {
					this.monitor.emit("orderFilled", { orderId: order.id, hash: result.txHash })
				}

				if (result.commitment) {
					const commitment = result.commitment as HexString
					this.bidStorage?.storeBid({
						commitment,
						extrinsicHash: (result.txHash as HexString) || undefined,
						success: result.success,
						error: result.error,
					})

					if (this.pendingRetractions.delete(commitment)) {
						this.logger.info({ commitment }, "OrderFilled arrived before bid was stored, retracting now")
						this.enqueueRetraction(commitment)
					}
				}

				return result
			} catch (error) {
				this.logger.error({ orderId: order.id, err: error }, "Order execution failed")
				throw error
			}
		})
	}

	private handleOrderFilledOnChain(commitment: HexString, filler: string, chainId: number): void {
		if (!this.bidStorage || !this.hyperbridge) {
			return
		}

		const bid = this.bidStorage.getBidByCommitment(commitment)
		if (!bid) {
			this.pendingRetractions.add(commitment)
			this.logger.debug(
				{ commitment, filler, chainId },
				"OrderFilled received before bid stored, deferring retraction",
			)
			return
		}

		if (bid.retracted) {
			this.logger.debug({ commitment }, "Bid already retracted, skipping")
			return
		}

		this.enqueueRetraction(commitment)
	}

	private enqueueRetraction(commitment: HexString): void {
		this.retractionQueue.add(async () => {
			try {
				this.logger.info({ commitment }, "Retracting bid after on-chain OrderFilled")

				const coprocessor = await this.hyperbridge!
				const result = await coprocessor.retractBid(commitment)

				if (result.success) {
					this.bidStorage!.markBidAsRetracted(commitment, result.extrinsicHash as HexString)
					this.logger.info({ commitment, retractHash: result.extrinsicHash }, "Bid retracted successfully")
				} else {
					this.logger.error({ commitment, error: result.error }, "Failed to retract bid")
				}
			} catch (error) {
				this.logger.error({ commitment, err: error }, "Error retracting bid")
			}
		})
	}
}
