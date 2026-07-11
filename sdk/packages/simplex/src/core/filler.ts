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
	type PhantomOrderEvent,
	orderCommitment,
	bytes32ToBytes20,
	type TokenInfo,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import type { Address } from "viem"
import pQueue from "p-queue"
import {
	BidStorageService,
	ChainClientManager,
	ContractInteractionService,
	DelegationService,
	RebalancingService,
} from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import type { SigningAccount } from "@/services/wallet"
import { hasPaymaster } from "@/services/paymaster"
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
	private phantomUnsubscribe: (() => void) | null = null
	// Last phantom bid commitment per phantom-order series — keyed by chain + the directed token
	// pair, NOT by chain alone. The pallet generates one phantom order per configured token pair, so
	// several are live on the same chain at once; a new interval's bid must only retract the previous
	// bid for the SAME pair, otherwise bidding on a second pair would retract the first pair's bid.
	private lastPhantomCommitmentByPair = new Map<string, HexString>()
	private hyperbridge: Promise<IntentsCoprocessor> | undefined = undefined
	private config: FillerConfig
	private configService: FillerConfigService
	private signer: SigningAccount
	private fillerAddress: HexString
	private logger = getLogger("intent-filler")

	constructor(
		chainConfigs: ChainConfig[],
		strategies: FillerStrategy[],
		config: FillerConfig,
		configService: FillerConfigService,
		chainClientManager: ChainClientManager,
		contractService: ContractInteractionService,
		signer: SigningAccount,
		rebalancingService?: RebalancingService,
		bidStorage?: BidStorageService,
	) {
		this.configService = configService
		this.signer = signer
		this.fillerAddress = this.signer.account.address
		this.chainClientManager = chainClientManager
		this.contractService = contractService
		this.rebalancingService = rebalancingService
		this.bidStorage = bidStorage
		this.monitor = new EventMonitor(chainConfigs, configService, this.chainClientManager, this.fillerAddress)
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
	 * Initializes the filler, including setting up EIP-7702 delegation and
	 * depositing the target amount to the EntryPoint on chains where solver
	 * selection is active. This should be called before start().
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
			this.delegationService = new DelegationService(this.chainClientManager, this.configService, this.signer)
			this.logger.info(
				{ chains: chainsWithSolverSelection },
				"Setting up EIP-7702 delegation on chains with solver selection",
			)
			const result = await this.delegationService.setupDelegationOnChains(chainsWithSolverSelection)
			if (!result.success) {
				const failedChains = Object.entries(result.results)
					.filter(([, ok]) => !ok)
					.map(([chain]) => chain)
				const allFailed = failedChains.length === chainsWithSolverSelection.length
				if (allFailed) {
					this.logger.error(
						{ results: result.results },
						"EIP-7702 delegation failed on all chains; shutting down",
					)
					throw new Error(
						`EIP-7702 delegation failed on all chains: ${failedChains.join(", ")}. Shutting down for restart.`,
					)
				}
				this.logger.warn(
					{ failedChains, results: result.results },
					"Some chains failed EIP-7702 delegation setup; continuing on remaining chains",
				)
			}

			// Ensure EntryPoint deposit covers target gas units on chains
			// that do NOT have the Circle Paymaster configured.
			// Chains with Circle Paymaster pay gas in USDC instead.
			// Paymaster permit is handled per-order inside buildCirclePaymasterData.
			const targetGasUnits = this.configService.getTargetGasUnits()
			for (const chain of chainsWithSolverSelection) {
				if (hasPaymaster(chain, this.configService)) {
					this.logger.info({ chain }, "Skipping EntryPoint deposit — paymaster available")
					continue
				}
				try {
					await this.contractService.topUpEntryPointDeposit(chain, targetGasUnits)
				} catch (err) {
					this.logger.error({ chain, err }, "Failed to deposit to EntryPoint at startup")
				}
			}
		}
	}

	/**
	 * Immediately enqueues retraction for all stale bids (older than maxAgeMs).
	 * Returns the number of bids queued for retraction.
	 */
	public async retractStaleBids(maxAgeMs = 60 * 60 * 1000): Promise<number> {
		if (!this.bidStorage || !this.hyperbridge) return 0
		const expired = this.bidStorage.getExpiredUnretractedBids(maxAgeMs)
		await this.sweepExpiredBids(maxAgeMs)
		return expired.length
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

		if (this.hyperbridge) {
			this.startPhantomBidding()
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

		if (this.phantomUnsubscribe) {
			this.phantomUnsubscribe()
			this.phantomUnsubscribe = null
		}

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

		// Disconnect shared Hyperbridge connection
		if (this.hyperbridge) {
			const service = await this.hyperbridge.catch(() => null)
			await service?.disconnect()
		}

		this.logger.info("All orders processed, filler stopped")
	}

	// Operations

	private async verifyOrderOnSource(order: Order): Promise<boolean> {
		if (order.inputs.length === 0) {
			this.logger.warn({ orderId: order.id }, "Order has no inputs, rejecting")
			return false
		}

		const sourceClient = this.chainClientManager.getPublicClient(order.source)
		const intentGatewayAddress = this.configService.getIntentGatewayAddress(order.source)
		const commitment = order.id as HexString

		try {
			const escrows = await Promise.all(
				order.inputs.map((input: TokenInfo) =>
					retryPromise(
						() =>
							sourceClient.readContract({
								address: intentGatewayAddress,
								abi: INTENT_GATEWAY_V2_ABI,
								functionName: "_orders",
								args: [commitment, bytes32ToBytes20(input.token) as Address],
							}) as Promise<bigint>,
						{
							maxRetries: 3,
							backoffMs: 250,
							logMessage: "Failed to read _orders on source chain",
						},
					),
				),
			)

			for (let i = 0; i < escrows.length; i++) {
				if (escrows[i] === 0n) {
					this.logger.warn(
						{
							orderId: order.id,
							source: order.source,
							inputIndex: i,
							token: order.inputs[i].token,
						},
						"Phantom commitment: source escrow missing for input, skipping order",
					)
					return false
				}
			}

			return true
		} catch (err) {
			this.logger.error(
				{ orderId: order.id, source: order.source, err },
				"Failed to verify source escrow, skipping order",
			)
			return false
		}
	}

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

				if (!this.configService.isUserAllowed(order.user, order.source)) {
					this.logger.debug(
						{ orderId: order.id, user: order.user, source: order.source },
						"Order user not in allowlist, skipping",
					)
					return
				}

				// Guard against phantom commitments: the off-chain order reconstruction
				// can mis-pair OrderPlaced logs with placeOrder calldata when a single tx
				// contains multiple placeOrder calls, yielding a commitment that has no
				// matching escrow on source. Reject those before bidding/filling.
				if (!(await this.verifyOrderOnSource(order))) {
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
					const fillableStrategies = [...canFillCache].filter(([, canFill]) => canFill)
					if (fillableStrategies.length === 0) {
						this.logger.debug(
							{ orderId: order.id, source: order.source, destination: order.destination },
							"Skipping cross-chain order: no strategy can fill it",
						)
						return
					}
					if (!fillableStrategies.some(([strategy]) => strategy.confirmationPolicy)) {
						this.logger.warn(
							{ orderId: order.id, source: order.source, destination: order.destination },
							"Skipping cross-chain order: no fillable strategy has a confirmation policy configured",
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
				const confirmStartMs = Date.now()

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
				const confirmDurationSec = (Date.now() - confirmStartMs) / 1000
				this.monitor.emit("orderTiming", {
					orderId: order.id,
					phase: "confirmation",
					durationSec: confirmDurationSec,
				})

				// Execute immediately
				if (evaluationResult) {
					this.executeOrder(
						order,
						evaluationResult.strategy,
						solverSelectionActive,
						inputUsdValue,
						evaluationResult.profitability,
					)
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

		const evalStartMs = Date.now()
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

		const evalDurationSec = (Date.now() - evalStartMs) / 1000
		this.monitor.emit("orderTiming", { orderId: order.id, phase: "evaluation", durationSec: evalDurationSec })

		if (validStrategies.length === 0) {
			this.logger.warn({ orderId: order.id }, "No profitable strategy found for order")
			this.monitor.emit("orderSkipped", { orderId: order.id, reason: "No profitable strategy" })
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

	private executeOrder(
		order: Order,
		bestStrategy: FillerStrategy,
		solverSelectionActive: boolean,
		inputUsdValue: Decimal,
		profitUsd: number,
	): void {
		// Get the chain-specific queue
		const chainQueue = this.chainQueues.get(getChainId(order.destination)!)
		if (!chainQueue) {
			this.logger.error({ chain: order.destination }, "No queue configured for chain")
			return
		}

		// Execute with the most profitable strategy using the chain-specific queue
		// This ensures transactions for the same chain are processed sequentially
		const queuedAtMs = Date.now()
		chainQueue.add(async () => {
			const queueDurationSec = (Date.now() - queuedAtMs) / 1000
			this.monitor.emit("orderTiming", { orderId: order.id, phase: "queue_wait", durationSec: queueDurationSec })

			this.logger.info(
				{ orderId: order.id, strategy: bestStrategy.name, chain: order.destination },
				"Executing order",
			)

			try {
				const execStartMs = Date.now()
				const hyperbridgeService = solverSelectionActive ? await this.hyperbridge : undefined
				const result = await bestStrategy.executeOrder(order, hyperbridgeService)
				const execDurationSec = (Date.now() - execStartMs) / 1000
				this.monitor.emit("orderTiming", {
					orderId: order.id,
					phase: "execution",
					durationSec: execDurationSec,
				})
				this.logger.info({ orderId: order.id, result }, "Order execution completed")

				if (result.success) {
					this.monitor.emit("orderFilled", {
						orderId: order.id,
						hash: result.txHash,
						volumeUsd: inputUsdValue.toNumber(),
						profitUsd,
						chainId: getChainId(order.source),
					})
				}
				this.monitor.emit("orderExecuted", {
					orderId: order.id,
					success: result.success,
					txHash: result.txHash,
					strategy: bestStrategy.name,
					commitment: result.commitment,
					error: result.error,
				})

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
		// Top up EntryPoint deposit if we were the filler, but only on chains
		// without any paymaster (paymaster chains pay gas in ERC-20 tokens).
		if (filler.toLowerCase() === this.fillerAddress.toLowerCase()) {
			const chain = `EVM-${chainId}`
			if (!hasPaymaster(chain, this.configService)) {
				const targetGasUnits = this.configService.getTargetGasUnits()
				this.contractService.topUpEntryPointDeposit(chain, targetGasUnits, 1_000_000n).catch((err) => {
					this.logger.error({ commitment, chain, err }, "Post-fill EntryPoint deposit top-up failed")
				})
			}
		}

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

	private startPhantomBidding(): void {
		if (!this.hyperbridge) return
		this.hyperbridge
			.then(async (coprocessor) => {
				this.phantomUnsubscribe = await coprocessor.subscribePhantomOrders((event) => {
					this.globalQueue.add(() => this.handlePhantomOrder(event, coprocessor))
				})
				this.logger.info("Phantom order subscription active")
			})
			.catch((err) => {
				this.logger.error({ err }, "Failed to start phantom order subscription")
			})
	}

	private async handlePhantomOrder(event: PhantomOrderEvent, coprocessor: IntentsCoprocessor): Promise<void> {
		const entryPointAddress = this.configService.getEntryPointAddress(`EVM-${getChainId(event.chain) ?? event.chain}`)
		if (!entryPointAddress) {
			this.logger.debug({ chain: event.chain }, "No entry point configured for phantom order chain, skipping")
			return
		}

		// Fetch the exact ABI-encoded order the pallet committed to from offchain storage.
		const phantomOrder = await coprocessor.fetchPhantomOrder(event.commitment)
		if (!phantomOrder) {
			this.logger.warn(
				{ commitment: event.commitment, chain: event.chain },
				"Phantom order not found in offchain storage — node may not be an offchain worker or order expired",
			)
			return
		}

		// Pick the strategy that actually handles this order's token pair — the same canFill matching
		// used for regular orders — then require it to support phantom quoting. (Selecting the first
		// strategy that merely has quotePhantomFill could pick one that doesn't handle this pair, e.g.
		// the stable strategy quoting an FX pair.)
		let strategy: FillerStrategy | undefined
		for (const candidate of this.strategies) {
			try {
				if (await candidate.canFill(phantomOrder)) {
					strategy = candidate
					break
				}
			} catch (err) {
				this.logger.error(
					{ err, commitment: event.commitment, strategy: candidate.name },
					"canFill check failed for phantom order",
				)
			}
		}
		if (!strategy) {
			this.logger.debug({ chain: event.chain }, "No strategy handles the phantom order's token pair, skipping")
			return
		}
		if (typeof strategy.quotePhantomFill !== "function") {
			this.logger.debug(
				{ chain: event.chain, strategy: strategy.name },
				"Matched strategy does not support phantom quoting, skipping",
			)
			return
		}

		let fillerOutputs: TokenInfo[] | null = null
		try {
			fillerOutputs = await strategy.quotePhantomFill(phantomOrder)
		} catch (err) {
			this.logger.warn({ err, commitment: phantomOrder.id, chain: event.chain }, "quotePhantomFill failed")
			return
		}

		if (!fillerOutputs || fillerOutputs.length === 0) {
			this.logger.debug({ chain: event.chain }, "Strategy declined phantom order")
			return
		}

		const solverAccountAddress = this.signer.account.address as HexString

		try {
			const { userOp } = await this.contractService.preparePhantomBidUserOp(
				phantomOrder,
				entryPointAddress,
				solverAccountAddress,
				fillerOutputs,
			)

			// Use event.commitment directly — re-deriving it from the decoded order risks parity
			// divergence if the encode round-trip doesn't perfectly reproduce the pallet's bytes.
			// When a previous interval's bid for THIS pair is still live, retract it and place the new
			// bid in one utility.batch so the old deposit is reclaimed even if the new bid fails. The
			// key is per (chain, token pair) so bidding on one pair never retracts another pair's bid.
			// Submissions are serialised inside IntentsCoprocessor (a single nonce-ordered queue), so a
			// new interval's bid can go out directly even when several pairs register in one block.
			const pairKey = `${event.chain}:${event.tokenA.toLowerCase()}:${event.tokenB.toLowerCase()}`
			const prevCommitment = this.lastPhantomCommitmentByPair.get(pairKey)
			const result =
				prevCommitment && prevCommitment !== event.commitment
					? await coprocessor.submitBidWithRetraction(prevCommitment, event.commitment, userOp)
					: await coprocessor.submitBid(event.commitment, userOp)
			if (result.success) {
				this.lastPhantomCommitmentByPair.set(pairKey, event.commitment)
				this.logger.info(
					{
						commitment: event.commitment,
						chain: event.chain,
						tokenA: event.tokenA,
						tokenB: event.tokenB,
						txHash: result.extrinsicHash,
						blockHash: result.blockHash,
					},
					"Phantom bid submitted",
				)
			} else {
				this.logger.warn(
					{ commitment: event.commitment, chain: event.chain, error: result.error },
					"Phantom bid rejected",
				)
			}
		} catch (err) {
			this.logger.error({ err, chain: event.chain }, "Failed to prepare or submit phantom bid")
		}
	}
}
