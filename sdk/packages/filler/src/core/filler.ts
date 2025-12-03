import { chainIds, retryPromise } from "@hyperbridge/sdk"
import { EventMonitor } from "./event-monitor"
import { FillerStrategy } from "@/strategies/base"
import { Order, FillerConfig, ChainConfig, DUMMY_PRIVATE_KEY, ADDRESS_ZERO, bytes20ToBytes32 } from "@hyperbridge/sdk"
import pQueue from "p-queue"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { CacheService } from "@/services/CacheService"
import { getLogger } from "@/services/Logger"

import { PublicClient } from "viem"
import { generatePrivateKey } from "viem/accounts"

export class IntentFiller {
	public monitor: EventMonitor
	private strategies: FillerStrategy[]
	private chainQueues: Map<number, pQueue>
	private globalQueue: pQueue
	private chainClientManager: ChainClientManager
	private contractService: ContractInteractionService
	private config: FillerConfig
	private configService: FillerConfigService
	private logger = getLogger("intent-filler")

	constructor(
		chainConfigs: ChainConfig[],
		strategies: FillerStrategy[],
		config: FillerConfig,
		configService: FillerConfigService,
		sharedCacheService?: CacheService,
	) {
		this.configService = configService
		this.chainClientManager = new ChainClientManager(configService)
		this.monitor = new EventMonitor(chainConfigs, configService, this.chainClientManager)
		this.strategies = strategies
		this.config = config

		this.contractService = new ContractInteractionService(
			this.chainClientManager,
			generatePrivateKey(),
			configService,
			sharedCacheService,
		)
		this.chainQueues = new Map()
		chainConfigs.forEach((chainConfig) => {
			// 1 order per chain at a time due to EVM constraints
			this.chainQueues.set(chainConfig.chainId, new pQueue({ concurrency: 1 }))
		})

		this.globalQueue = new pQueue({
			concurrency: config.maxConcurrentOrders || 5,
		})

		// Set up event handlers
		this.monitor.on("newOrder", ({ order }) => {
			this.handleNewOrder(order)
		})
	}

	public start(): void {
		this.monitor.startListening()
	}

	public stop(): void {
		this.monitor.stopListening()

		// Wait for all queues to complete
		const promises = []
		this.chainQueues.forEach((queue) => {
			promises.push(queue.onIdle())
		})
		promises.push(this.globalQueue.onIdle())

		Promise.all(promises).then(() => {
			this.logger.info("All orders processed, filler stopped")
		})
	}

	// Operations

	private handleNewOrder(order: Order): void {
		// Use the global queue for the initial analysis
		// This can happen in parallel for PublicClient orders
		this.globalQueue.add(async () => {
			this.logger.info({ orderId: order.id }, "New order detected")
			try {
				const sourceClient = this.chainClientManager.getPublicClient(order.sourceChain)
				const orderValue = await this.contractService.getTokenUsdValue(order)
				let currentConfirmations = await retryPromise(
					() =>
						sourceClient.getTransactionConfirmations({
							hash: order.transactionHash!,
						}),
					{
						maxRetries: 3,
						backoffMs: 250,
						logMessage: "Failed to get initial transaction confirmations",
					},
				)
				const requiredConfirmations = this.config.confirmationPolicy.getConfirmationBlocks(
					chainIds[order.sourceChain as keyof typeof chainIds],
					orderValue.inputUsdValue.toNumber(),
				)
				this.logger.info(
					{ orderId: order.id, requiredConfirmations, currentConfirmations },
					"Order confirmation requirements",
				)

				while (currentConfirmations < requiredConfirmations) {
					await new Promise((resolve) => setTimeout(resolve, 300)) // Wait 300ms
					currentConfirmations = await retryPromise(
						() =>
							sourceClient.getTransactionConfirmations({
								hash: order.transactionHash!,
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

				this.evaluateAndExecuteOrder(order)
			} catch (error) {
				this.logger.error({ orderId: order.id, err: error }, "Error processing order")
			}
		})
	}

	private evaluateAndExecuteOrder(order: Order): void {
		this.globalQueue.add(async () => {
			try {
				const eligibleStrategies = await Promise.all(
					this.strategies.map(async (strategy) => {
						const canFill = await strategy.canFill(order)
						if (!canFill) return null

						const profitability = await strategy.calculateProfitability(order)
						return { strategy, profitability }
					}),
				)

				const validStrategies = eligibleStrategies
					.filter((s): s is NonNullable<typeof s> => s !== null && s.profitability > 0n)
					.sort((a, b) => Number(b.profitability) - Number(a.profitability))

				if (validStrategies.length === 0) {
					this.logger.warn({ orderId: order.id }, "No profitable strategy found for order")
					return
				}

				// Get the chain-specific queue
				const chainQueue = this.chainQueues.get(chainIds[order.destChain as keyof typeof chainIds]!)
				if (!chainQueue) {
					this.logger.error({ chain: order.destChain }, "No queue configured for chain")
					return
				}

				// Execute with the most profitable strategy using the chain-specific queue
				// This ensures transactions for the same chain are processed sequentially
				chainQueue.add(async () => {
					const bestStrategy = validStrategies[0].strategy
					this.logger.info(
						{ orderId: order.id, strategy: bestStrategy.name, chain: order.destChain },
						"Executing order",
					)

					try {
						const result = await bestStrategy.executeOrder(order)
						this.logger.info({ orderId: order.id, result }, "Order execution completed")
						if (result.success) {
							this.monitor.emit("orderFilled", { orderId: order.id, hash: result.txHash })
						}
						return result
					} catch (error) {
						this.logger.error({ orderId: order.id, err: error }, "Order execution failed")
						throw error
					}
				})
			} catch (error) {
				this.logger.error({ orderId: order.id, err: error }, "Error processing order")
			}
		})
	}
}
