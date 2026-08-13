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
import { ChainClientManager, ContractInteractionService, DelegationService, RebalancingService } from "@/services"
import type { BidStore } from "@/data/types"
import { SharedHyperbridgeSource } from "@/scanner/registry"
import type { HyperbridgeSource, OrderSource, Subscription } from "@/scanner/types"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger, type Logger , moduleLogger} from "@/services/Logger"
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
	private bidStorage?: BidStore
	private retractionQueue: pQueue
	private paused = false
	private pendingRetractions = new Set<string>()
	private rebalancingInterval?: NodeJS.Timeout
	private retractionSweepInterval?: NodeJS.Timeout
	private stopPhantomPolling: (() => void) | null = null
	// Last phantom bid commitment per chain. The pallet bundles every configured pair into a single
	// order per interval, so one commitment per chain is live at a time and a new interval's bid
	// retracts exactly the one it replaces.
	private lastPhantomCommitmentByChain = new Map<string, HexString>()
	private hyperbridge: Promise<IntentsCoprocessor> | undefined = undefined
	private config: FillerConfig
	private configService: FillerConfigService
	private signer: SigningAccount
	private fillerAddress: HexString
	private logger: Logger
	private hyperbridgeSource: HyperbridgeSource
	private phantomSubscription?: Subscription

	constructor(
		chainConfigs: ChainConfig[],
		strategies: FillerStrategy[],
		config: FillerConfig,
		configService: FillerConfigService,
		chainClientManager: ChainClientManager,
		contractService: ContractInteractionService,
		signer: SigningAccount,
		rebalancingService?: RebalancingService,
		bidStorage?: BidStore,
		sources: { orders?: OrderSource; hyperbridge?: HyperbridgeSource } = {},
	) {
		this.logger = moduleLogger(configService.loggers, "intent-filler")
		this.configService = configService
		this.signer = signer
		this.fillerAddress = this.signer.account.address
		this.chainClientManager = chainClientManager
		this.contractService = contractService
		this.rebalancingService = rebalancingService
		this.bidStorage = bidStorage
		this.monitor = new EventMonitor(
			chainConfigs,
			configService,
			this.chainClientManager,
			this.fillerAddress,
			sources.orders,
		)
		this.hyperbridgeSource = sources.hyperbridge ?? new SharedHyperbridgeSource(configService.loggers)
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
			this.handleOrderFilledOnChain(commitment as HexString, filler, chainId).catch((err) => {
				// The retraction sweep still picks this bid up on its next cycle.
				this.logger.error({ commitment, err }, "Failed to handle on-chain fill")
			})
		})
	}

	/**
	 * Initializes the filler, including setting up EIP-7702 delegation and
	 * depositing the target amount to the EntryPoint on chains where solver
	 * selection is active. This should be called before start().
	 */
	/** Whether the given chain id is configured for watch-only (monitor, never fill). */
	private isChainWatchOnly(chainId: number): boolean {
		const watchOnly = this.config.watchOnly
		return typeof watchOnly === "object" && watchOnly !== null && watchOnly[chainId] === true
	}

	public async initialize(): Promise<void> {
		const chains = await this.solverSelectionChains(this.configService.getConfiguredChainIds())
		// Boot tolerates partial failure: as long as one chain delegated, the
		// filler is useful. Only a total failure is fatal.
		await this.setupSolverSelection(chains, { requireAll: false })
	}

	/** Chains that both fill and have solver selection active, as state machine ids. */
	private async solverSelectionChains(chainIds: number[]): Promise<string[]> {
		const active: string[] = []
		for (const chainId of chainIds) {
			// Watch-only chains never fill, so they never need EIP-7702 delegation
			// or an EntryPoint deposit. Skipping them lets a signerless watch-only
			// filler (which runs on a throwaway key) start without attempting a
			// delegation that would fail on the unfunded account.
			if (this.isChainWatchOnly(chainId)) continue
			const chain = `EVM-${chainId}`
			if (await this.contractService.isSolverSelectionActive(chain)) {
				active.push(chain)
				this.logger.info({ chain }, "Solver selection is active on chain")
			}
		}
		return active
	}

	/**
	 * Delegates the filler EOA via EIP-7702 and tops up EntryPoint deposits.
	 *
	 * `requireAll` distinguishes the two callers. At boot a partial failure is
	 * survivable and only a total one shuts the filler down. When a single chain
	 * is being added at runtime, any failure must reject the call so the caller
	 * can roll the chain back — a chain left scanning but unable to bid would
	 * look configured while silently declining every order on it.
	 */
	private async setupSolverSelection(chains: string[], { requireAll }: { requireAll: boolean }): Promise<void> {
		if (chains.length === 0 || !this.hyperbridge) return

		this.delegationService ??= new DelegationService(this.chainClientManager, this.configService, this.signer)
		this.logger.info({ chains }, "Setting up EIP-7702 delegation on chains with solver selection")

		const result = await this.delegationService.setupDelegationOnChains(chains)
		if (!result.success) {
			const failedChains = Object.entries(result.results)
				.filter(([, ok]) => !ok)
				.map(([chain]) => chain)
			if (requireAll) {
				throw new Error(`EIP-7702 delegation failed on ${failedChains.join(", ")}`)
			}
			if (failedChains.length === chains.length) {
				this.logger.error({ results: result.results }, "EIP-7702 delegation failed on all chains; shutting down")
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
		// that do NOT have any paymaster (Circle or Simplex) configured.
		// Chains with a paymaster pay gas in stablecoins instead.
		// Paymaster authorization is handled per-order inside buildPaymasterAndData.
		const targetGasUnits = this.configService.getTargetGasUnits()
		for (const chain of chains) {
			if (hasPaymaster(chain, this.configService)) {
				this.logger.info({ chain }, "Skipping EntryPoint deposit — paymaster available")
				continue
			}
			try {
				await this.contractService.topUpEntryPointDeposit(chain, targetGasUnits)
			} catch (err) {
				// Non-fatal on both paths: the deposit is topped up again after
				// every fill, so a transient RPC failure here self-heals.
				this.logger.error({ chain, err }, "Failed to deposit to EntryPoint at startup")
			}
		}
	}

	/**
	 * Brings a chain into the running filler: gives it an execution queue,
	 * delegates on it if solver selection is active, then starts its scanner.
	 *
	 * Ordering matters. The queue exists before the scanner starts, because an
	 * order arriving on a chain with no queue would throw at execution time; and
	 * the scanner starts last, so a chain that fails delegation never sees an
	 * order at all. Any failure rolls the queue back and rethrows.
	 */
	public async addChain(chainConfig: ChainConfig): Promise<void> {
		const { chainId } = chainConfig
		if (this.chainQueues.has(chainId)) {
			throw new Error(`Chain ${chainId} is already running`)
		}
		// 1 order per chain at a time due to EVM constraints
		this.chainQueues.set(chainId, new pQueue({ concurrency: 1 }))
		try {
			await this.setupSolverSelection(await this.solverSelectionChains([chainId]), { requireAll: true })
			await this.monitor.addChain(chainId)
		} catch (error) {
			this.chainQueues.delete(chainId)
			throw error
		}
		this.logger.info({ chainId }, "Chain added to the running filler")
	}

	/**
	 * Removes a chain. Stops the scanner first so nothing new is queued, then
	 * drains what is already in flight — dropping the queue under a running fill
	 * would strand it mid-execution.
	 */
	public async removeChain(chainId: number): Promise<void> {
		await this.monitor.removeChain(chainId)
		const queue = this.chainQueues.get(chainId)
		if (queue) {
			await queue.onIdle()
			this.chainQueues.delete(chainId)
		}
		if (this.config.watchOnly) delete this.config.watchOnly[chainId]
		this.logger.info({ chainId }, "Chain removed from the running filler")
	}

	/**
	 * Immediately enqueues retraction for all bids due for it: stale bids (older than maxAgeMs)
	 * and dead bids (order already filled) of any age.
	 * Returns the number of bids queued for retraction.
	 */
	public async retractStaleBids(maxAgeMs = 60 * 60 * 1000): Promise<number> {
		return this.sweepExpiredBids(maxAgeMs)
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
	 * Stops analysing and filling new orders while keeping the event monitor
	 * alive; in-flight fills complete. Orders arriving while paused are dropped,
	 * not queued — resuming does not replay them.
	 */
	public pause(): void {
		if (this.paused) return
		this.paused = true
		this.globalQueue.pause()
		this.logger.warn("Filler paused — monitoring continues, new orders are not analysed or filled")
	}

	public resume(): void {
		if (!this.paused) return
		this.paused = false
		this.globalQueue.start()
		this.logger.info("Filler resumed")
	}

	public isPaused(): boolean {
		return this.paused
	}

	/** Takes effect immediately: evaluateOrder reads the map on every order. */
	public setWatchOnly(chainId: number, value: boolean): void {
		if (this.config.watchOnly === undefined) {
			this.config.watchOnly = {}
		}
		this.config.watchOnly[chainId] = value
	}

	public getWatchOnly(): Record<number, boolean> {
		return this.config.watchOnly ?? {}
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
				this.monitor.emit("rebalanceExecuted", {
					transferCount: result.transfers.length,
					executedCount: result.executedTransfers.length,
					success: result.success,
				})
			} else if (result.transfers.length === 0) {
				this.logger.debug("No rebalancing needed")
			}
		} catch (error) {
			this.logger.error({ error }, "Portfolio rebalancing failed")
			this.monitor.emit("rebalanceExecuted", {
				success: false,
				error: error instanceof Error ? error.message : String(error),
			})
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

		this.logger.info("Periodic retraction sweep started (every 5 minutes; 1h TTL, dead bids swept next cycle)")
	}

	/** Enqueues retraction for every due bid; returns how many were queued. */
	private async sweepExpiredBids(maxAgeMs: number): Promise<number> {
		if (!this.bidStorage || !this.hyperbridge) {
			return 0
		}

		const expired = await this.bidStorage.expiredUnretracted(maxAgeMs)
		if (expired.length === 0) {
			return 0
		}

		this.logger.info({ count: expired.length }, "Sweeping expired unretracted bids")

		for (const bid of expired) {
			this.enqueueRetraction(bid.commitment as HexString)
		}
		return expired.length
	}

	public async stop(): Promise<void> {
		this.monitor.stopListening()

		this.phantomSubscription?.close()
		this.phantomSubscription = undefined
		if (this.stopPhantomPolling) {
			this.stopPhantomPolling()
			this.stopPhantomPolling = null
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

		// A paused queue never resolves onIdle; drop anything still pending
		// (matching pause semantics) and unblock the drain below.
		if (this.paused) {
			this.globalQueue.clear()
			this.globalQueue.start()
			this.paused = false
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
		if (this.paused) {
			this.logger.debug({ orderId: order.id }, "Filler is paused — dropping new order")
			return
		}
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

				// Guard against phantom commitments: a decode bug or malformed event
				// would yield a commitment that has no matching escrow on source.
				// Reject those before bidding/filling.
				if (!(await this.verifyOrderOnSource(order))) {
					return
				}

				// Confirmations are counted with BFT-quorum semantics across the
				// operator's configured endpoints, so with independent providers a
				// single compromised or reorged provider cannot vouch for inclusion
				// depth on cross-chain orders.
				const sourceQuorumClient = this.chainClientManager.getQuorumClient(order.source)
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

				// Single-provider setups keep the tight 300ms poll; quorum setups
				// fan every poll out to all providers, so poll less aggressively
				// to stay within their rate limits.
				const confirmationPollMs = sourceQuorumClient.size > 1 ? 1000 : 300
				const waitForConfirmations = async (): Promise<void> => {
					// Nothing to wait for: same-chain orders (and zero-valued curve
					// points) require no confirmations, and the quorum read they'd
					// otherwise run gains nothing — it would only gate the fill on
					// third-party RPC availability, where a transient QuorumError
					// rejects the surrounding Promise.all and drops the order.
					if (requiredConfirmations <= 0) return

					let currentConfirmations = await retryPromise(
						() =>
							sourceQuorumClient.getTransactionConfirmations({
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
						await new Promise((resolve) => setTimeout(resolve, confirmationPollMs))
						if (abortController.signal.aborted) return
						currentConfirmations = await retryPromise(
							() =>
								sourceQuorumClient.getTransactionConfirmations({
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
		const isWatchOnly = destChainId !== undefined && this.isChainWatchOnly(destChainId)

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
			// Emitted as a skip, not a bespoke "detected" event: every other
			// not-filled path reports `orderSkipped`, and nothing consumed the old
			// event — so watch-only orders were absent from the activity feed and
			// the skip metric entirely.
			this.monitor.emit("orderSkipped", { orderId: order.id, reason: "watch-only" })
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
					// Awaited, not fired and forgotten: the pendingRetractions check below
					// reads its own write, and a deposit whose bid record never landed is a
					// deposit the retraction sweep will never find.
					await this.bidStorage?.store({
						commitment,
						extrinsicHash: (result.txHash as HexString) || undefined,
						success: result.success,
						error: result.error,
					})

					if (this.pendingRetractions.delete(commitment)) {
						this.logger.info({ commitment }, "OrderFilled arrived before bid was stored, retracting now")
						await this.bidStorage?.markDead(commitment)
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

	private async handleOrderFilledOnChain(commitment: HexString, filler: string, chainId: number): Promise<void> {
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

		// Flag the deferral before reading, not after. The bid write on the fill path
		// is now awaited, so it can land in the gap between these two statements; with
		// the old read-then-add order that bid would sit in pendingRetractions with
		// nobody left to claim it, and wait out the full stale-bid TTL. Adding first
		// means whichever side observes the other's write does the retraction, and
		// `delete` returning a boolean keeps exactly one of them doing it.
		this.pendingRetractions.add(commitment)

		const bid = await this.bidStorage.byCommitment(commitment)
		if (!bid) {
			this.logger.debug(
				{ commitment, filler, chainId },
				"OrderFilled received before bid stored, deferring retraction",
			)
			return
		}
		if (!this.pendingRetractions.delete(commitment)) {
			// The fill path claimed it while we were reading.
			return
		}

		if (bid.retracted) {
			this.logger.debug({ commitment }, "Bid already retracted, skipping")
			return
		}

		// The order is filled, so the bid is dead weight from here on. Should the immediate
		// retraction below not confirm, the sweep re-attempts dead bids on its next cycle
		// instead of waiting out the stale-bid TTL.
		await this.bidStorage.markDead(commitment)
		this.enqueueRetraction(commitment)
	}

	private enqueueRetraction(commitment: HexString): void {
		this.retractionQueue.add(async () => {
			try {
				// Both OrderFilled and the periodic sweep can enqueue the same commitment; a fresh
				// read at dequeue time skips the duplicate instead of paying for a BidNotFound.
				if ((await this.bidStorage!.byCommitment(commitment))?.retracted) {
					return
				}

				this.logger.info({ commitment }, "Retracting bid")

				const coprocessor = await this.hyperbridge!
				const result = await coprocessor.retractBid(commitment)

				if (result.success) {
					await this.bidStorage!.markRetracted(commitment, (result.extrinsicHash as HexString) ?? null)
					this.logger.info({ commitment, retractHash: result.extrinsicHash }, "Bid retracted successfully")
				} else if (result.error?.includes("BidNotFound")) {
					// Terminal, not retryable: bids only leave the pallet by retraction, so "no bid"
					// means there is nothing left to reclaim — an earlier submission of ours landed,
					// someone retracted manually, or the placement never landed. Anything else seeds
					// a zombie the sweep re-retracts forever.
					await this.bidStorage!.markRetracted(commitment, null)
					this.logger.debug({ commitment }, "No bid on chain, marked as retracted")
				} else if (result.pending) {
					// Our extrinsic is still in the Hyperbridge tx pool. Resubmitting can only bounce
					// off it (1014) or land a duplicate; leave the bid as-is and let the sweep confirm
					// — if the pooled retraction lands, the next attempt's BidNotFound closes it out.
					this.logger.debug(
						{ commitment, error: result.error },
						"Retraction already in flight, deferring to sweep",
					)
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
		const wsUrl = this.configService.getHyperbridgeWsUrl()
		if (!wsUrl) return
		this.hyperbridge
			.then((coprocessor) => {
				// Reads come from the shared poller — every filler used to re-read every
				// Hyperbridge block itself. Bids still go through this instance's own
				// coprocessor, which holds its substrate key.
				this.phantomSubscription = this.hyperbridgeSource.subscribe(wsUrl, {
					onPhantomOrder: (order) => {
						// The queued promise is nobody's return value, so an escaping throw would be an
						// unhandled rejection — which this process has no handler for and Node turns into
						// an exit. Contain it here so a bad phantom order can never take the filler down.
						void this.globalQueue
							.add(() => this.handlePhantomOrder(order, coprocessor))
							.catch((err) =>
								this.logger.error(
									{ err, chain: order.chain, commitment: order.commitment },
									"Unhandled error while processing a phantom order",
								),
							)
					},
					onError: (err) => this.logger.warn({ err }, "Phantom order poll failed, will retry"),
				})
				this.logger.info("Phantom order polling active")
			})
			.catch((err) => {
				this.logger.error({ err }, "Failed to start phantom order polling")
			})
	}

	/**
	 * Quotes one leg of a bundled phantom order. Legs are positional, so narrowing the order to
	 * `inputs[index]` and `output.assets[index]` produces the single pair order the strategies
	 * already know how to price. `quotePhantomFill` does its own canFill gating and returns null
	 * for a leg it does not handle, so there is no pre-check here — one canFill per strategy,
	 * not two. The synthetic leg carries no id so nothing keys the shared TTL caches with
	 * per-interval throwaway entries.
	 */
	private async quotePhantomLeg(order: Order, index: number, chain: string): Promise<TokenInfo | null> {
		const leg: Order = {
			...order,
			id: undefined,
			inputs: [order.inputs[index]],
			output: { ...order.output, assets: [order.output.assets[index]] },
		}

		for (const candidate of this.strategies) {
			if (typeof candidate.quotePhantomFill !== "function") continue
			try {
				const outputs = await candidate.quotePhantomFill(leg)
				if (outputs?.length) return outputs[0]
			} catch (err) {
				this.logger.warn(
					{ err, commitment: order.id, chain, index, strategy: candidate.name },
					"Phantom leg quote failed",
				)
			}
		}

		return null
	}

	private async handlePhantomOrder(event: PhantomOrderEvent, coprocessor: IntentsCoprocessor): Promise<void> {
		// A phantom bid is a public quote — it advertises that this filler stands ready to fill that
		// chain's pairs at the quoted rate. The pallet registers one order per chain *it* knows about,
		// which is a superset of what any single operator runs, and the strategies price legs off the
		// shared asset registry rather than the operator's config, so an unconfigured chain quotes
		// happily. Bidding there advertises liquidity no real order could ever draw on: without RPCs
		// or a bundler for the chain, the fill path cannot even see the order, let alone fill it.
		// Watch-only chains are the same story — `evaluateOrder` refuses to fill them by design.
		const chainId = getChainId(event.chain)
		if (chainId === undefined || !this.configService.getConfiguredChainIds().includes(chainId)) {
			this.logger.debug({ chain: event.chain }, "Phantom order chain is not configured, skipping")
			return
		}
		if (this.isChainWatchOnly(chainId)) {
			this.logger.debug({ chain: event.chain }, "Phantom order chain is watch-only, skipping")
			return
		}

		const entryPointAddress = this.configService.getEntryPointAddress(`EVM-${chainId}`)
		if (!entryPointAddress) {
			this.logger.debug({ chain: event.chain }, "No entry point configured for phantom order chain, skipping")
			return
		}

		// Fetch the exact ABI-encoded order the pallet committed to from offchain storage. The read
		// throws rather than returning empty when the node does not serve the offchain RPC at all,
		// which is the common misconfiguration — a public endpoint runs safe methods only.
		let phantomOrder: Order | null
		try {
			phantomOrder = await coprocessor.fetchPhantomOrder(event.commitment)
		} catch (err) {
			this.logger.error(
				{ err, commitment: event.commitment, chain: event.chain },
				"Could not read the phantom order from offchain storage — the Hyperbridge node must run with " +
					"--enable-offchain-indexing=true and expose offchain_localStorageGet (--rpc-methods=unsafe)",
			)
			return
		}
		if (!phantomOrder) {
			this.logger.warn(
				{ commitment: event.commitment, chain: event.chain },
				"Phantom order not found in offchain storage — node may not be an offchain worker or order expired",
			)
			return
		}

		// Every leg of every configured pair rides in this one order, so quote leg by leg —
		// concurrently, because legs are independent and the bid has to land inside the on-chain
		// bid window (as few as 5 blocks), which a sequential walk over a large order would miss.
		// A leg no strategy handles is quoted at zero instead of sinking the whole order, and legs
		// belonging to different strategies each get the one that actually prices them.
		const quotes = await Promise.all(
			phantomOrder.output.assets.map((_, index) => this.quotePhantomLeg(phantomOrder, index, event.chain)),
		)
		const fillerOutputs: TokenInfo[] = quotes.map(
			(quote, index) => quote ?? { token: phantomOrder.output.assets[index].token, amount: 0n },
		)

		if (fillerOutputs.every((output) => output.amount === 0n)) {
			this.logger.debug({ chain: event.chain }, "No strategy quoted any leg of the phantom order")
			return
		}

		const solverAccountAddress = this.signer.account.address as HexString

		try {
			const { userOp } = await this.contractService.preparePhantomBidUserOp(
				phantomOrder,
				entryPointAddress,
				solverAccountAddress,
				fillerOutputs,
				this.config.acceptedSourceChains,
			)

			// Use event.commitment directly — re-deriving it from the decoded order risks parity
			// divergence if the encode round-trip doesn't perfectly reproduce the pallet's bytes.
			// When the previous interval's bid on this chain is still live, retract it and place the
			// new bid in one utility.batch so the old deposit is reclaimed even if the new bid fails.
			const prevCommitment = this.lastPhantomCommitmentByChain.get(event.chain)
			const result =
				prevCommitment && prevCommitment !== event.commitment
					? await coprocessor.submitBidWithRetraction(prevCommitment, event.commitment, userOp)
					: await coprocessor.submitBid(event.commitment, userOp)
			if (result.success) {
				this.lastPhantomCommitmentByChain.set(event.chain, event.commitment)
				this.logger.info(
					{
						commitment: event.commitment,
						chain: event.chain,
						quotedLegs: fillerOutputs.filter((output) => output.amount > 0n).length,
						legs: fillerOutputs.length,
						txHash: result.extrinsicHash,
						blockHash: result.blockHash,
					},
					"Phantom bid submitted",
				)
			} else if (result.pending) {
				// The extrinsic reached the tx pool but inclusion wasn't observed — it will almost
				// certainly land. Record the commitment so the next interval's batch retracts it;
				// if it never lands, that retraction degrades to a harmless trailing BidNotFound.
				this.lastPhantomCommitmentByChain.set(event.chain, event.commitment)
				this.logger.info(
					{ commitment: event.commitment, chain: event.chain, error: result.error },
					"Phantom bid in flight, inclusion not yet observed",
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
