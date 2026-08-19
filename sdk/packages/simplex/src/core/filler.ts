import { keccakAsU8a } from "@polkadot/util-crypto"
import { EventMonitor } from "./event-monitor"
import type { FillerStrategy } from "@/strategies/base"
import {
	type Order,
	type FillerConfig,
	type ChainConfig,
	getChainId,
	retryPromise,
	type HexString,
	IntentsCoprocessor,
	type PhantomOrderEvent,
	bytes32ToBytes20,
	type TokenInfo,
	type PhantomBid,
} from "@hyperbridge/sdk"
import { parseChainKey } from "@/config/interpolated-curve"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import type { Address } from "viem"
import pQueue from "p-queue"
import { type ChainClientManager, type ContractInteractionService, DelegationService, type RebalancingService } from "@/services"
import type { BidStore } from "@/data/types"
import type { HyperbridgeScanner, OrderScanner, Subscription } from "@/scanner/types"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { type Logger , moduleLogger} from "@/services/Logger"
import type { Signer } from "@/services/wallet"
import { hasPaymaster } from "@/services/paymaster"
import { Decimal } from "decimal.js"

/** How long to wait for a Hyperbridge connection before giving up on it. */
const HYPERBRIDGE_CONNECT_TIMEOUT_MS = 30_000

/** One chain's phantom bid, quoted and built, waiting to ride in the interval's batch. */
interface PreparedPhantomBid {
	chain: string
	/** Legs that got a non-zero quote, and how many the order carried — for logging only. */
	quotedLegs: number
	legs: number
	bid: PhantomBid
}

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
	private stopping = false
	private pendingRetractions = new Set<string>()
	private rebalancingInterval?: NodeJS.Timeout
	private initialRebalanceTimer?: NodeJS.Timeout
	private retractionSweepInterval?: NodeJS.Timeout
	private stopPhantomPolling: (() => void) | null = null
	// Last phantom bid commitment per chain. The pallet bundles every configured pair into a single
	// order per interval, so one commitment per chain is live at a time and a new interval's bid
	// retracts exactly the one it replaces.
	private lastPhantomCommitmentByChain = new Map<string, HexString>()
	private hyperbridge: Promise<IntentsCoprocessor> | undefined = undefined
	private hyperbridgeEndpoint?: { wsUrl: string; substrateKey: string }
	/** The ApiPromise behind `hyperbridge` — ours, so stop() can disconnect it. */
	// biome-ignore lint/suspicious/noExplicitAny: polkadot api type kept out of the public surface
	private hyperbridgeApi: any

	/**
	 * The filler's Hyperbridge connection, so other services can share it rather than opening a
	 * second socket to the same node.
	 */
	get hyperbridgeConnection(): Promise<IntentsCoprocessor> | undefined {
		return this.hyperbridge
	}
	private config: FillerConfig
	private configService: FillerConfigService
	private signer: Signer
	private fillerAddress: HexString
	private logger: Logger
	private hyperbridgeScanner?: HyperbridgeScanner
	private phantomSubscription?: Subscription

	constructor(
		chainConfigs: ChainConfig[],
		strategies: FillerStrategy[],
		config: FillerConfig,
		configService: FillerConfigService,
		chainClientManager: ChainClientManager,
		contractService: ContractInteractionService,
		signer: Signer,
		scanners: { orders: OrderScanner; hyperbridge?: HyperbridgeScanner },
		rebalancingService?: RebalancingService,
		bidStorage?: BidStore,
	) {
		this.logger = moduleLogger(configService.loggers, "intent-filler")
		this.configService = configService
		this.signer = signer
		this.fillerAddress = this.signer.address
		this.chainClientManager = chainClientManager
		this.contractService = contractService
		this.rebalancingService = rebalancingService
		this.bidStorage = bidStorage
		this.monitor = new EventMonitor(chainConfigs, configService, this.fillerAddress, scanners.orders)
		this.hyperbridgeScanner = scanners.hyperbridge
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
			// Deferred to initialize(): connecting here would make a boot-time
			// failure a permanently rejected promise the fill path trips over
			// forever — a solver that scans and evaluates but can never bid, while
			// status() reports healthy. initialize() awaits the connect, so an
			// unreachable node rejects Simplex.start() loudly instead.
			this.hyperbridgeEndpoint = { wsUrl: hyperbridgeWsUrl, substrateKey }
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

	/**
	 * Opens the bidding connection to Hyperbridge, owned by this filler.
	 *
	 * Built like HyperbridgeScanner.start: our WsProvider, raced against a
	 * timeout because ApiPromise.create retries a dead endpoint forever, and the
	 * provider is disconnected when the race is lost so nothing keeps dialling
	 * with no owner. Awaited from initialize(), so an unreachable node fails the
	 * boot instead of producing a solver that can never bid.
	 */
	private async connectHyperbridge(): Promise<void> {
		if (!this.hyperbridgeEndpoint) return
		const { wsUrl, substrateKey } = this.hyperbridgeEndpoint

		const { ApiPromise, WsProvider } = await import("@polkadot/api")
		const provider = new WsProvider(wsUrl)
		const api = await Promise.race([
			ApiPromise.create({
				provider,
				typesBundle: { spec: { nexus: { hasher: keccakAsU8a }, gargantua: { hasher: keccakAsU8a } } },
			}),
			new Promise<never>((_, reject) =>
				setTimeout(
					() => reject(new Error(`Timed out connecting to Hyperbridge at ${wsUrl}`)),
					HYPERBRIDGE_CONNECT_TIMEOUT_MS,
				).unref(),
			),
		]).catch(async (error) => {
			await provider.disconnect().catch(() => {})
			throw error
		})

		this.hyperbridgeApi = api
		// Signing stays here: `fromApi` marks the connection as ours, so only this
		// filler's stop() closes it.
		this.hyperbridge = Promise.resolve(IntentsCoprocessor.fromApi(api, substrateKey))
	}

	public async initialize(): Promise<void> {
		await this.connectHyperbridge()
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

	/**
	 * Forgets a chain's watch-only flag entirely, rather than setting it false.
	 *
	 * Used when an add is rolled back: leaving `{ [chainId]: false }` behind would
	 * report a watch-only state for a chain that is no longer configured, and the
	 * config sync would persist it.
	 */
	public clearWatchOnly(chainId: number): void {
		if (this.config.watchOnly) delete this.config.watchOnly[chainId]
	}

	public getWatchOnly(): Record<number, boolean> {
		return this.config.watchOnly ?? {}
	}

	/**
	 * Start periodic rebalancing checks.
	 * Checks every 5 minutes for triggers and executes rebalancing if needed.
	 */
	private startRebalancing(): void {
		// Run initial check after 30 seconds (to let the filler start up). Tracked so
		// it cannot fire after stop() has resolved.
		this.initialRebalanceTimer = setTimeout(() => {
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
		this.stopping = true
		this.monitor.stopListening()

		// The bidding connection is ours (fromApi does not own it), so nothing
		// else will close this socket.
		await this.hyperbridgeApi?.disconnect().catch(() => {})
		this.hyperbridgeApi = undefined
		this.hyperbridge = undefined

		this.phantomSubscription?.close()
		this.phantomSubscription = undefined
		if (this.stopPhantomPolling) {
			this.stopPhantomPolling()
			this.stopPhantomPolling = null
		}

		// Stop rebalancing interval
		if (this.initialRebalanceTimer) {
			clearTimeout(this.initialRebalanceTimer)
			this.initialRebalanceTimer = undefined
		}
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
				// Orders destined to chains this filler cannot fill on are expected
				// mainnet traffic, not faults — other fillers cover other lanes. They
				// used to fall through to the cache check below and be dropped with a
				// misleading "Shared cache is not initialized" ERROR, since the cache
				// is only ever populated for configured, non-watch-only chains.
				const destinationChainId = parseChainKey(order.destination)
				if (
					destinationChainId === null ||
					!this.configService.getConfiguredChainIds().includes(destinationChainId)
				) {
					this.logger.debug(
						{ orderId: order.id, destination: order.destination },
						"Order destination is not a configured chain, skipping",
					)
					return
				}
				if (this.isChainWatchOnly(destinationChainId)) {
					this.logger.debug(
						{ orderId: order.id, destination: order.destination },
						"Order destination is watch-only, skipping",
					)
					return
				}

				// Early check: if solver selection is active, ensure hyperbridge is configured.
				// With the destination confirmed configured and filling above, a missing
				// entry now really is an initialization bug.
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

		// A partial fill is exempt from the profit floor. It collects no `order.fees`
		// and hands over less than the order asked for, so what it earns is the margin
		// the operator already built into the pair's own curve — a figure the engine
		// cannot see and therefore scores at or near zero. The decision to fill at
		// that curve was made when the curve was configured.
		const fillsPartially = this.fillsPartially(order)
		const validStrategies = eligibleStrategies
			.filter((s): s is NonNullable<typeof s> => s !== null && (s.profitability > 0 || fillsPartially))
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

	/**
	 * Whether the strategy's evaluation concluded in a deliberate partial fill.
	 *
	 * Read from a flag the strategy sets only once it has an answer, never inferred
	 * from the outputs it cached along the way: those are written before the profit
	 * gates run, so an order the strategy went on to REFUSE still has a plan sitting
	 * in the cache. Inferring from it exempted those refusals from the floor below
	 * and filled them.
	 */
	private fillsPartially(order: Order): boolean {
		if (!order.id) return false
		return this.contractService.cacheService.isPartialFill(order.id)
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

				// Persist the bid FIRST, before any telemetry. By this point the bid is
				// already on Hyperbridge holding a deposit, and the only way to reclaim
				// it is to retract it — which the sweep can only do for bids it can find
				// here. Anything between the submission and this write is something that
				// can strand money: a consumer's event listener throwing, an
				// operator-supplied store rejecting on a connection blip, a disk error.
				if (result.commitment) {
					const commitment = result.commitment as HexString
					await this.bidStorage?.store({
						commitment,
						extrinsicHash: (result.txHash as HexString) || undefined,
						success: result.success,
						pending: result.pending === true,
						error: result.error,
					})

					if (this.pendingRetractions.delete(commitment)) {
						this.logger.info({ commitment }, "OrderFilled arrived before bid was stored, retracting now")
						this.enqueueRetraction(commitment)
						await this.bidStorage?.markDead(commitment)
					}
				}

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

				return result
			} catch (error) {
				this.logger.error({ orderId: order.id, err: error }, "Order execution failed")
				throw error
			}
			// The queued promise is nobody's return value, so the rethrow above would be an
			// unhandled rejection — which this process has no handler for and Node turns
			// into an exit, stopping the retraction sweep for every other outstanding bid.
			// Same guard the phantom path already applies.
		}).catch((err) => this.logger.error({ orderId: order.id, err }, "Order execution task failed"))
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

		// Retract first, flag second. markDead only buys a faster *retry* — the sweep
		// re-attempts dead bids without waiting out the TTL — so letting a failed
		// flag-write suppress the retraction itself inverts the intent.
		this.enqueueRetraction(commitment)
		await this.bidStorage.markDead(commitment)
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
		const scanner = this.hyperbridgeScanner
		if (!scanner) return
		this.hyperbridge
			.then((coprocessor) => {
				// connect() can resolve long after stop() was called against a slow
				// endpoint; installing the poller then would leave it running forever.
				if (this.stopping) return
				// Reads come from the shared poller — every filler used to re-read every
				// Hyperbridge block itself. Bids still go through this instance's own
				// coprocessor, which holds its substrate key.
				this.phantomSubscription = scanner.subscribe({
					onPhantomOrders: (orders: PhantomOrderEvent[]) => {
						// The queued promise is nobody's return value, so an escaping throw would be an
						// unhandled rejection — which this process has no handler for and Node turns into
						// an exit. Contain it here so a bad phantom order can never take the filler down.
						void this.globalQueue
							.add(() => this.handlePhantomOrders(orders, coprocessor))
							.catch((err) =>
								this.logger.error(
									{ err, chains: orders.map((order) => order.chain) },
									"Unhandled error while processing phantom orders",
								),
							)
					},
					onError: (err: unknown) => this.logger.warn({ err }, "Phantom order poll failed, will retry"),
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

	/**
	 * Quotes and builds one chain's phantom bid, ready to be batched with the rest of the interval's.
	 * Returns null when the chain is skipped, nothing quoted, or the userOp could not be built —
	 * one chain dropping out never costs the others their bid.
	 */
	/**
	 * Quotes and builds one chain's phantom bid, ready to be batched with the rest of the interval's.
	 * Returns null when the chain is skipped, nothing quoted, or the userOp could not be built —
	 * one chain dropping out never costs the others their bid.
	 */
	private async preparePhantomBid(
		event: PhantomOrderEvent,
		coprocessor: IntentsCoprocessor,
	): Promise<PreparedPhantomBid | null> {
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
			return null
		}
		if (this.isChainWatchOnly(chainId)) {
			this.logger.debug({ chain: event.chain }, "Phantom order chain is watch-only, skipping")
			return null
		}

		const entryPointAddress = this.configService.getEntryPointAddress(`EVM-${chainId}`)
		if (!entryPointAddress) {
			this.logger.debug({ chain: event.chain }, "No entry point configured for phantom order chain, skipping")
			return null
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
			return null
		}
		if (!phantomOrder) {
			this.logger.warn(
				{ commitment: event.commitment, chain: event.chain },
				"Phantom order not found in offchain storage — node may not be an offchain worker or order expired",
			)
			return null
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
			return null
		}

		const solverAccountAddress = this.signer.address as HexString

		try {
			const { userOp } = await this.contractService.preparePhantomBidUserOp(
				phantomOrder,
				entryPointAddress,
				solverAccountAddress,
				fillerOutputs,
				this.config.acceptedSourceChains,
				// Positions are declared per chain because the bid is: the tokenIds that back a quote
				// on this chain are the ones held here.
				this.config.uniswapV4PositionsByChain?.[event.chain],
			)

			// Use event.commitment directly — re-deriving it from the decoded order risks parity
			// divergence if the encode round-trip doesn't perfectly reproduce the pallet's bytes.
			// When the previous interval's bid on this chain is still live it rides along as a
			// retraction, so the old deposit comes back in the same extrinsic.
			const prevCommitment = this.lastPhantomCommitmentByChain.get(event.chain)
			return {
				chain: event.chain,
				quotedLegs: fillerOutputs.filter((output) => output.amount > 0n).length,
				legs: fillerOutputs.length,
				bid: {
					commitment: event.commitment,
					userOp,
					retractCommitment:
						prevCommitment && prevCommitment !== event.commitment ? prevCommitment : undefined,
				},
			}
		} catch (err) {
			this.logger.error({ err, chain: event.chain }, "Failed to prepare phantom bid")
			return null
		}
	}

	/**
	 * Bids on every phantom order registered in one block, in a single extrinsic.
	 *
	 * The pallet registers one order per configured chain in the same block, so this is the whole
	 * interval's set. Quoting stays per chain and runs concurrently; only the submission is shared.
	 * One bid per chain used to mean one extrinsic per chain, each waiting for inclusion behind the
	 * last because submissions are serialised on the account nonce — so the final chain's bid was
	 * many blocks behind the first, against a bid window measured in tens of blocks.
	 */
	private async handlePhantomOrders(events: PhantomOrderEvent[], coprocessor: IntentsCoprocessor): Promise<void> {
		const prepared = (await Promise.all(events.map((event) => this.preparePhantomBid(event, coprocessor)))).filter(
			(entry): entry is PreparedPhantomBid => entry !== null,
		)
		if (prepared.length === 0) return

		const result = await coprocessor.submitPhantomBids(prepared.map((entry) => entry.bid))

		const landed: string[] = []
		prepared.forEach((entry, index) => {
			const outcome = result.bids[index]
			if (outcome?.success) {
				landed.push(entry.chain)
				this.lastPhantomCommitmentByChain.set(entry.chain, entry.bid.commitment)
				return
			}
			if (result.pending) {
				// The extrinsic reached the tx pool but inclusion wasn't observed — it will almost
				// certainly land. Record the commitment so the next interval's batch retracts it;
				// if it never lands, that retraction degrades to a harmless trailing BidNotFound.
				this.lastPhantomCommitmentByChain.set(entry.chain, entry.bid.commitment)
				return
			}
			this.logger.warn(
				{ commitment: entry.bid.commitment, chain: entry.chain, error: outcome?.error ?? result.error },
				"Phantom bid rejected",
			)
		})

		if (result.pending) {
			this.logger.info(
				{ bids: prepared.length, chains: prepared.map((entry) => entry.chain), error: result.error },
				"Phantom bids in flight, inclusion not yet observed",
			)
			return
		}

		this.logger.info(
			{
				bids: prepared.length,
				landed: landed.length,
				chains: landed,
				quotedLegs: prepared.reduce((sum, entry) => sum + entry.quotedLegs, 0),
				legs: prepared.reduce((sum, entry) => sum + entry.legs, 0),
				txHash: result.extrinsicHash,
				blockHash: result.blockHash,
				error: result.error,
			},
			landed.length > 0 ? "Phantom bids submitted" : "Phantom bid batch landed no bids",
		)
	}
}
