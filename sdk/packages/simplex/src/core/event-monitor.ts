import { EventEmitter } from "events"
import {
	ChainConfig,
	EvmChain,
	IChain,
	Order,
	TronChain,
	orderCommitment,
	hexToString,
	retryPromise,
	DecodedOrderPlacedLog,
	HexString,
	tronChainIds,
	IEvmChain,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { decodeFunctionData } from "viem"
import { ChainClientManager } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import { QuorumPublicClient } from "@/services/QuorumPublicClient"
import { Mutex } from "async-mutex"

export interface ReconstructDeps {
	getPlaceOrderCalldata: (txHash: string, occurrenceIndex: number) => Promise<HexString>
	onError?: (err: unknown, log: DecodedOrderPlacedLog, occurrenceIndex: number) => void
}

/**
 * Pure reconstruction of `OrderPlaced` logs into `Order` structs with commitments.
 * Groups logs by transaction hash and pairs the K-th log in a tx with the K-th
 * placeOrder calldata supplied by `deps.getPlaceOrderCalldata(txHash, K)`.
 */
export async function reconstructOrdersFromLogs(
	logs: DecodedOrderPlacedLog[],
	deps: ReconstructDeps,
): Promise<{ order: Order; transactionHash: string }[]> {
	const logsByTx = new Map<string, { log: DecodedOrderPlacedLog; occurrenceIndex: number }[]>()
	for (const log of logs) {
		const txHash = log.transactionHash as string
		const bucket = logsByTx.get(txHash) ?? []
		bucket.push({ log, occurrenceIndex: bucket.length })
		logsByTx.set(txHash, bucket)
	}

	const out: { order: Order; transactionHash: string }[] = []

	for (const [transactionHash, entries] of logsByTx) {
		for (const { log: decodedLog, occurrenceIndex } of entries) {
			try {
				let order: Order = {
					user: decodedLog.args.user,
					source: hexToString(decodedLog.args.source) as HexString,
					destination: hexToString(decodedLog.args.destination) as HexString,
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
						call: "0x",
					},
					output: {
						beneficiary: "0x0000000000000000000000000000000000000000",
						assets: decodedLog.args.outputs.map(
							(output: { token: HexString; amount: bigint }) => ({
								token: output.token,
								amount: output.amount,
							}),
						),
						call: "0x",
					},
					inputs: decodedLog.args.inputs.map(
						(input: { token: HexString; amount: bigint }) => ({
							token: input.token,
							amount: input.amount,
						}),
					),
				}

				const placeOrderCallInput = await deps.getPlaceOrderCalldata(transactionHash, occurrenceIndex)

				const decodedCalldata = decodeFunctionData({
					abi: INTENT_GATEWAY_V2_ABI,
					data: placeOrderCallInput as HexString,
				})?.args?.[0] as Order

				order.output.beneficiary = decodedCalldata.output.beneficiary as `0x${string}`
				order.output.call = decodedCalldata.output.call as HexString
				order.predispatch.call = decodedCalldata.predispatch.call as HexString
				order.id = orderCommitment(order)

				out.push({ order, transactionHash })
			} catch (error) {
				if (deps.onError) deps.onError(error, decodedLog, occurrenceIndex)
			}
		}
	}

	return out
}

export class EventMonitor extends EventEmitter {
	private chains: Map<number, IEvmChain> = new Map()
	private quorumClients: Map<number, QuorumPublicClient> = new Map()
	private listening: boolean = false
	private configService: FillerConfigService
	private fillerAddress: string
	private logger = getLogger("event-monitor")
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
		this.configService = configService
		this.fillerAddress = fillerAddress.toLowerCase()

		chainConfigs.forEach((config) => {
			const chainName = `EVM-${config.chainId}`
			const chainParams = {
				stateMachineId: chainName,
				chainId: config.chainId,
				rpcUrl: this.configService.getRpcUrl(chainName),
				host: this.configService.getHostAddress(chainName),
				consensusStateId: this.configService.getConsensusStateId(chainName),
			}
			const chain = EvmChain.fromParams(chainParams)
			this.chains.set(config.chainId, chain as IEvmChain)
			this.scanningMutexes.set(config.chainId, new Mutex())

			const rpcUrls = this.configService.getRpcUrls(chainName)
			this.quorumClients.set(config.chainId, new QuorumPublicClient(config.chainId, rpcUrls))
			if (rpcUrls.length > 1) {
				this.logger.info(
					{ chainId: config.chainId, providerCount: rpcUrls.length },
					"Quorum log scanning enabled",
				)
			}
		})
	}

	public async startListening(): Promise<void> {
		if (this.listening) return
		this.listening = true

		const gatewayEvents = INTENT_GATEWAY_V2_ABI.filter(
			(item) =>
				item.type === "event" &&
				(item.name === "OrderPlaced" || item.name === "OrderFilled" || item.name === "PartialFill"),
		)

		for (const [chainId, chain] of this.chains.entries()) {
			try {
				const intentGatewayAddress = this.configService.getIntentGatewayAddress(`EVM-${chainId}`)

				const quorumClient = this.quorumClients.get(chainId)
				if (!quorumClient) {
					throw new Error(`Chain ${chainId} has no quorum client`)
				}

				const startBlock = await retryPromise(() => quorumClient.getBlockNumber(), {
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed to get start block number",
				})
				this.lastScannedBlock.set(chainId, startBlock - 1n)

				this.logger.info({ chainId, startBlock }, "Initializing block scanner")

				const scanInterval = setInterval(async () => {
					const mutex = this.scanningMutexes.get(chainId)
					if (!mutex) return

					if (mutex.isLocked()) {
						return
					}

					await mutex.runExclusive(async () => {
						try {
							await this.scanBlocks(chainId, chain, quorumClient, intentGatewayAddress, gatewayEvents)
						} catch (error) {
							this.logger.error({ chainId, err: error }, "Error in block scanner")
						}
					})
				}, 1000)

				this.blockScanIntervals.set(chainId, scanInterval)

				this.logger.info({ chainId }, "Started monitoring for new orders and fills")
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Failed to start block scanner")
			}
		}
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
		chain: IEvmChain,
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

		this.logger.debug(
			{ chainId, fromBlock, toBlock, gap: Number(toBlock - fromBlock) },
			"Scanning blocks",
		)

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
		const filledLogs = logs.filter(
			(l: any) => l.eventName === "OrderFilled" || l.eventName === "PartialFill",
		)

		if (placedLogs.length > 0) {
			this.logger.info(
				{ chainId, fromBlock, toBlock, eventCount: placedLogs.length },
				"Found OrderPlaced events in block scan",
			)
			await this.processOrderPlacedLogs(chainId, chain, placedLogs)
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

	private async processOrderPlacedLogs(chainId: number, chain: IEvmChain, logs: any[]): Promise<void> {
		const results = await reconstructOrdersFromLogs(logs as DecodedOrderPlacedLog[], {
			getPlaceOrderCalldata: (txHash, occurrenceIndex) => {
				const sourceFromTxHash = (logs as DecodedOrderPlacedLog[]).find(
					(l) => l.transactionHash === txHash,
				)
				const source = sourceFromTxHash
					? (hexToString(sourceFromTxHash.args.source) as HexString)
					: undefined
				const intentGatewayAddress = this.configService.getIntentGatewayAddress(source!)
				return chain.getPlaceOrderCalldata!(txHash, intentGatewayAddress, occurrenceIndex)
			},
			onError: (err, decodedLog, occurrenceIndex) => {
				this.logger.error({ err, log: decodedLog, occurrenceIndex }, "Error parsing event log")
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
