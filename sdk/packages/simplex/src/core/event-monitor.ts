import { EventEmitter } from "events"
import {
	ChainConfig,
	Order,
	orderCommitment,
	normalizeStateMachineId,
	retryPromise,
	DecodedOrderPlacedLog,
	HexString,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { ChainClientManager } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger, type Logger , moduleLogger} from "@/services/Logger"
import { QuorumPublicClient } from "@/services/QuorumPublicClient"
import { Mutex } from "async-mutex"

export interface ReconstructDeps {
	onError?: (err: unknown, log: DecodedOrderPlacedLog) => void
}

/**
 * Pure reconstruction of `OrderPlaced` logs into `Order` structs with
 * commitments. Every log carries the complete order — including both call
 * payloads — so each order is rebuilt from its log alone.
 */
export function reconstructOrdersFromLogs(
	logs: DecodedOrderPlacedLog[],
	deps: ReconstructDeps = {},
): { order: Order; transactionHash: string }[] {
	const out: { order: Order; transactionHash: string }[] = []

	for (const decodedLog of logs) {
		try {
			const { predispatchCall, outputCall } = decodedLog.args
			if (predispatchCall === undefined || outputCall === undefined) {
				throw new Error("OrderPlaced log is missing its call payloads; pre-#1092 gateways are not supported")
			}

			const order: Order = {
				user: decodedLog.args.user,
				source: normalizeStateMachineId(decodedLog.args.source) as HexString,
				destination: normalizeStateMachineId(decodedLog.args.destination) as HexString,
				deadline: decodedLog.args.deadline,
				nonce: decodedLog.args.nonce,
				fees: decodedLog.args.fees,
				session: decodedLog.args.session,
				predispatch: {
					assets: decodedLog.args.predispatch.map(
						(predispatch: { token: HexString; amount: bigint }) => ({
							token: predispatch.token,
							amount: predispatch.amount,
						}),
					),
					call: predispatchCall,
				},
				output: {
					beneficiary: decodedLog.args.beneficiary,
					assets: decodedLog.args.outputs.map((output: { token: HexString; amount: bigint }) => ({
						token: output.token,
						amount: output.amount,
					})),
					call: outputCall,
				},
				inputs: decodedLog.args.inputs.map((input: { token: HexString; amount: bigint }) => ({
					token: input.token,
					amount: input.amount,
				})),
			}

			order.id = orderCommitment(order)

			out.push({ order, transactionHash: decodedLog.transactionHash as string })
		} catch (error) {
			if (deps.onError) deps.onError(error, decodedLog)
		}
	}

	return out
}

/** Gateway events every chain scanner subscribes to. */
const GATEWAY_EVENTS = INTENT_GATEWAY_V2_ABI.filter(
	(item) =>
		item.type === "event" &&
		(item.name === "OrderPlaced" || item.name === "OrderFilled" || item.name === "PartialFill"),
)

export class EventMonitor extends EventEmitter {
	private quorumClients: Map<number, QuorumPublicClient> = new Map()
	private listening: boolean = false
	private configService: FillerConfigService
	private clientManager: ChainClientManager
	private fillerAddress: string
	private logger: Logger
	private lastScannedBlock: Map<number, bigint> = new Map()
	private blockScanIntervals: Map<number, NodeJS.Timeout> = new Map()
	private scanningMutexes: Map<number, Mutex> = new Map()

	constructor(
		chainConfigs: ChainConfig[],
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		fillerAddress: HexString,
	) {
		super()
		this.logger = moduleLogger(configService.loggers, "event-monitor")
		this.configService = configService
		this.clientManager = clientManager
		this.fillerAddress = fillerAddress.toLowerCase()

		chainConfigs.forEach((config) => this.registerChain(config.chainId))
	}

	/** Caches the shared quorum client and scan mutex for a chain. */
	private registerChain(chainId: number): void {
		this.scanningMutexes.set(chainId, new Mutex())

		// Shared quorum client over the operator's configured endpoints, also
		// used by the cross-chain confirmation waiter in the filler core.
		const quorumClient = this.clientManager.getQuorumClient(`EVM-${chainId}`)
		this.quorumClients.set(chainId, quorumClient)
		if (quorumClient.size > 1) {
			this.logger.info(
				{ chainId, providerCount: quorumClient.size, threshold: quorumClient.threshold },
				"Quorum log scanning enabled",
			)
		}
	}

	public async startListening(): Promise<void> {
		if (this.listening) return
		this.listening = true

		for (const chainId of this.quorumClients.keys()) {
			try {
				await this.startScanner(chainId)
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Failed to start block scanner")
			}
		}
	}

	/**
	 * Starts (or restarts) one chain's scan loop from the current head.
	 *
	 * `fromBlock` lets a restart resume an existing cursor instead of skipping
	 * whatever landed while the scanner was down — an RPC swap must not open a
	 * hole in the scanned range.
	 */
	private async startScanner(chainId: number, fromBlock?: bigint): Promise<void> {
		const intentGatewayAddress = this.configService.getIntentGatewayAddress(`EVM-${chainId}`)

		const quorumClient = this.quorumClients.get(chainId)
		if (!quorumClient) {
			throw new Error(`Chain ${chainId} has no quorum client`)
		}

		const startBlock =
			fromBlock ??
			(await retryPromise(() => quorumClient.getBlockNumber(), {
				maxRetries: 3,
				backoffMs: 250,
				logMessage: "Failed to get start block number",
			})) - 1n
		this.lastScannedBlock.set(chainId, startBlock)

		this.logger.info({ chainId, startBlock }, "Initializing block scanner")

		const scanInterval = setInterval(async () => {
			const mutex = this.scanningMutexes.get(chainId)
			if (!mutex) return

			if (mutex.isLocked()) {
				return
			}

			await mutex.runExclusive(async () => {
				try {
					await this.scanBlocks(chainId, quorumClient, intentGatewayAddress, GATEWAY_EVENTS)
				} catch (error) {
					this.logger.error({ chainId, err: error }, "Error in block scanner")
				}
			})
		}, 1000)

		this.blockScanIntervals.set(chainId, scanInterval)

		this.logger.info({ chainId }, "Started monitoring for new orders and fills")
	}

	/** Stops one chain's scan loop, returning the cursor so a restart can resume it. */
	private async stopScanner(chainId: number): Promise<bigint | undefined> {
		const interval = this.blockScanIntervals.get(chainId)
		if (interval) {
			clearInterval(interval)
			this.blockScanIntervals.delete(chainId)
		}
		// Let an in-flight scan finish so the cursor we return is settled.
		await this.scanningMutexes.get(chainId)?.runExclusive(async () => {})
		return this.lastScannedBlock.get(chainId)
	}

	/** Begins scanning a chain added after boot. */
	public async addChain(chainId: number): Promise<void> {
		if (this.quorumClients.has(chainId)) {
			throw new Error(`Chain ${chainId} is already monitored`)
		}
		this.registerChain(chainId)
		if (this.listening) await this.startScanner(chainId)
	}

	/** Stops scanning a chain and forgets its cursor. */
	public async removeChain(chainId: number): Promise<void> {
		await this.stopScanner(chainId)
		this.quorumClients.delete(chainId)
		this.scanningMutexes.delete(chainId)
		this.lastScannedBlock.delete(chainId)
		this.logger.info({ chainId }, "Stopped monitoring chain")
	}

	/**
	 * Rebuilds a chain's client against its current RPC URLs, preserving the scan
	 * cursor. `ChainClientManager.invalidate` must have run first — this re-reads
	 * the quorum client it dropped.
	 */
	public async rebuildChain(chainId: number): Promise<void> {
		if (!this.quorumClients.has(chainId)) {
			throw new Error(`Chain ${chainId} is not monitored`)
		}
		const cursor = await this.stopScanner(chainId)
		this.quorumClients.delete(chainId)
		this.registerChain(chainId)
		if (this.listening) await this.startScanner(chainId, cursor)
		this.logger.info({ chainId, cursor }, "Rebuilt chain scanner on new endpoints")
	}

	private isBlockRangeError(error: any): boolean {
		const message = String(error?.message || error?.details || "")
		return (
			message.includes("block range extends beyond current head block") ||
			message.includes("invalid block range params")
		)
	}

	private async scanBlocks(
		chainId: number,
		quorumClient: QuorumPublicClient,
		intentGatewayAddress: `0x${string}`,
		gatewayEvents: any[],
	): Promise<void> {
		const lastScanned = this.lastScannedBlock.get(chainId)
		if (!lastScanned) return

		const currentBlock = await retryPromise(() => quorumClient.getBlockNumber(), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: "Failed to get current block number",
		})

		if (currentBlock <= lastScanned) return

		const fromBlock = lastScanned + 1n
		const maxBlockRange = 1000n
		const toBlock = fromBlock + maxBlockRange > currentBlock ? currentBlock : fromBlock + maxBlockRange

		this.logger.debug({ chainId, fromBlock, toBlock, gap: Number(toBlock - fromBlock) }, "Scanning blocks")

		let logs: any[]
		try {
			logs = await retryPromise(
				() =>
					quorumClient.getLogs({
						address: intentGatewayAddress,
						events: gatewayEvents,
						fromBlock,
						toBlock,
					}),
				{
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed to get gateway event logs",
				},
			)
		} catch (error: any) {
			// RPC hasn't indexed these blocks yet — don't advance the cursor,
			// just wait for the next tick to retry.
			if (this.isBlockRangeError(error)) return
			throw error
		}

		const placedLogs = logs.filter((l: any) => l.eventName === "OrderPlaced")
		const filledLogs = logs.filter((l: any) => l.eventName === "OrderFilled" || l.eventName === "PartialFill")

		if (placedLogs.length > 0) {
			this.logger.info(
				{ chainId, fromBlock, toBlock, eventCount: placedLogs.length },
				"Found OrderPlaced events in block scan",
			)
			this.processOrderPlacedLogs(placedLogs)
		}

		if (filledLogs.length > 0) {
			this.logger.info(
				{ chainId, fromBlock, toBlock, eventCount: filledLogs.length },
				"Found OrderFilled events in block scan",
			)
			this.processOrderFilledLogs(chainId, filledLogs)
		}

		this.lastScannedBlock.set(chainId, toBlock)
	}

	private processOrderPlacedLogs(logs: any[]): void {
		const results = reconstructOrdersFromLogs(logs as DecodedOrderPlacedLog[], {
			onError: (err, decodedLog) => {
				this.logger.error({ err, log: decodedLog }, "Error parsing event log")
			},
		})

		for (const { order, transactionHash } of results) {
			this.logger.info({ orderId: order.id, txHash: transactionHash }, "New order detected")
			this.emit("newOrder", { order, transactionHash })
		}
	}

	private processOrderFilledLogs(chainId: number, logs: any[]): void {
		for (const log of logs) {
			try {
				const commitment = log.args?.commitment as HexString | undefined
				const filler = log.args?.filler as string | undefined

				if (!commitment) {
					this.logger.warn({ log }, "OrderFilled log missing commitment")
					continue
				}

				if (filler?.toLowerCase() !== this.fillerAddress) {
					continue
				}

				this.logger.info({ chainId, commitment, filler }, "OrderFilled event detected for this filler")
				this.emit("orderFilledOnChain", { commitment, filler, chainId })
			} catch (error) {
				this.logger.error({ err: error, log }, "Error parsing OrderFilled log")
			}
		}
	}

	public async stopListening(): Promise<void> {
		this.listening = false

		for (const [chainId, interval] of this.blockScanIntervals.entries()) {
			clearInterval(interval)
			this.logger.info({ chainId }, "Stopped block scanner")
		}
		this.blockScanIntervals.clear()

		const mutexPromises = Array.from(this.scanningMutexes.values()).map((mutex) =>
			mutex.runExclusive(async () => {
				// Empty function - just wait for any ongoing operations to complete
			}),
		)
		await Promise.allSettled(mutexPromises)

		this.scanningMutexes.clear()
		this.lastScannedBlock.clear()
	}
}
