import { getChainId, retryPromise, type HexString } from "@hyperbridge/sdk"
import { EventMonitor } from "./event-monitor"
import { FillerStrategy } from "@/strategies/base"
import { OrderV2, FillerConfig, ChainConfig } from "@hyperbridge/sdk"
import pQueue from "p-queue"
import { ChainClientManager, ContractInteractionService, DelegationService, HyperbridgeService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"

export class IntentFiller {
	public monitor: EventMonitor
	private strategies: FillerStrategy[]
	private chainQueues: Map<number, pQueue>
	private globalQueue: pQueue
	private chainClientManager: ChainClientManager
	private contractService: ContractInteractionService
	private delegationService?: DelegationService
	private hyperbridge: Promise<HyperbridgeService> | undefined = undefined
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
	) {
		this.configService = configService
		this.privateKey = privateKey
		this.chainClientManager = chainClientManager
		this.contractService = contractService
		this.monitor = new EventMonitor(chainConfigs, configService, this.chainClientManager)
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

		const hyperbridgeWsUrl = configService.getHyperbridgeWsUrl()
		const substrateKey = configService.getSubstratePrivateKey()
		if (hyperbridgeWsUrl && substrateKey) {
			this.hyperbridge = HyperbridgeService.create(hyperbridgeWsUrl, substrateKey)
		}

		// Set up event handlers
		this.monitor.on("newOrder", ({ order }) => {
			this.handleNewOrder(order)
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
	}

	public stop(): void {
		this.monitor.stopListening()

		// Wait for all queues to complete
		const promises: Promise<void>[] = []
		this.chainQueues.forEach((queue) => {
			promises.push(queue.onIdle())
		})
		promises.push(this.globalQueue.onIdle())

		Promise.all(promises).then(async () => {
			// Disconnect shared Hyperbridge connection
			if (this.hyperbridge) {
				const service = await this.hyperbridge.catch(() => null)
				await service?.disconnect()
			}

			this.logger.info("All orders processed, filler stopped")
		})
	}

	// Operations

	private handleNewOrder(order: OrderV2): void {
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
				const orderValue = await this.contractService.getTokenUsdValue(order)
				const requiredConfirmations = this.config.confirmationPolicy.getConfirmationBlocks(
					getChainId(order.source)!,
					orderValue.inputUsdValue.toNumber(),
				)

				// Run confirmation waiting and evaluation in parallel
				const waitForConfirmations = async (): Promise<void> => {
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
				}

				// Run confirmation and evaluation in parallel
				const [, evaluationResult] = await Promise.all([
					waitForConfirmations(),
					this.evaluateOrder(order),
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
		order: OrderV2,
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
				const canFill = await strategy.canFill(order)
				if (!canFill) return null

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
			{ orderId: order.id, strategy: validStrategies[0].strategy.name, profitability: validStrategies[0].profitability.toString() },
			"Order evaluation complete - profitable strategy found",
		)

		return validStrategies[0]
	}

	private executeOrder(
		order: OrderV2,
		bestStrategy: FillerStrategy,
		solverSelectionActive: boolean,
	): void {
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
				const hyperbridgeService = solverSelectionActive ? await this.hyperbridge : undefined
				const result = await bestStrategy.executeOrder(order, hyperbridgeService)
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
	}
}
