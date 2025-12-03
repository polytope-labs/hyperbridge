import { EventEmitter } from "events"
import { ChainConfig, Order, orderCommitment, hexToString, DecodedOrderPlacedLog, retryPromise } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { PublicClient } from "viem"
import { ChainClientManager } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import { Mutex } from "async-mutex"

export class EventMonitor extends EventEmitter {
	private clients: Map<number, PublicClient> = new Map()
	private listening: boolean = false
	private clientManager: ChainClientManager
	private configService: FillerConfigService
	private logger = getLogger("event-monitor")
	private lastScannedBlock: Map<number, bigint> = new Map()
	private blockScanIntervals: Map<number, NodeJS.Timeout> = new Map()
	private scanningMutexes: Map<number, Mutex> = new Map()

	constructor(chainConfigs: ChainConfig[], configService: FillerConfigService, clientManager: ChainClientManager) {
		super()
		this.configService = configService
		this.clientManager = clientManager

		chainConfigs.forEach((config) => {
			const chainName = `EVM-${config.chainId}`
			const client = this.clientManager.getPublicClient(chainName)
			this.clients.set(config.chainId, client)
			this.scanningMutexes.set(config.chainId, new Mutex())
		})
	}

	public async startListening(): Promise<void> {
		if (this.listening) return
		this.listening = true

		for (const [chainId, client] of this.clients.entries()) {
			try {
				const orderPlacedEvent = INTENT_GATEWAY_ABI.find(
					(item) => item.type === "event" && item.name === "OrderPlaced",
				)
				const intentGatewayAddress = this.configService.getIntentGatewayAddress(`EVM-${chainId}`)

				const startBlock = await retryPromise(() => client.getBlockNumber(), {
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
							await this.scanBlocks(chainId, client, intentGatewayAddress, orderPlacedEvent)
						} catch (error) {
							this.logger.error({ chainId, err: error }, "Error in block scanner")
						}
					})
				}, 1000)

				this.blockScanIntervals.set(chainId, scanInterval)

				this.logger.info({ chainId }, "Started monitoring for new orders")
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Failed to start block scanner")
			}
		}
	}

	private async scanBlocks(
		chainId: number,
		client: PublicClient,
		intentGatewayAddress: `0x${string}`,
		orderPlacedEvent: any,
	): Promise<void> {
		const lastScanned = this.lastScannedBlock.get(chainId)
		if (!lastScanned) return

		const currentBlock = await retryPromise(() => client.getBlockNumber(), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: "Failed to get current block number",
		})

		if (currentBlock > lastScanned) {
			const fromBlock = lastScanned + 1n
			const toBlock = currentBlock

			const maxBlockRange = 1000n
			const actualToBlock = fromBlock + maxBlockRange > toBlock ? toBlock : fromBlock + maxBlockRange

			this.logger.debug(
				{ chainId, fromBlock, toBlock: actualToBlock, gap: Number(actualToBlock - fromBlock) },
				"Scanning blocks",
			)

			const logs = await retryPromise(
				() =>
					client.getLogs({
						address: intentGatewayAddress,
						event: orderPlacedEvent,
						fromBlock,
						toBlock: actualToBlock,
					}),
				{
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed to get logs for block scan",
				},
			)

			if (logs.length > 0) {
				this.logger.info(
					{ chainId, fromBlock, toBlock: actualToBlock, eventCount: logs.length },
					"Found events in block scan",
				)
				this.processLogs(logs)
			}

			// Update lastScannedBlock only after successful processing
			// This is protected by the mutex, so no race condition
			this.lastScannedBlock.set(chainId, actualToBlock)
		}
	}

	private processLogs(logs: any[]): void {
		for (const log of logs) {
			try {
				const decodedLog = log as unknown as DecodedOrderPlacedLog
				const order: Order = {
					id: orderCommitment({
						id: "",
						user: decodedLog.args.user,
						sourceChain: hexToString(decodedLog.args.sourceChain),
						destChain: hexToString(decodedLog.args.destChain),
						deadline: decodedLog.args.deadline,
						nonce: decodedLog.args.nonce,
						fees: decodedLog.args.fees,
						outputs: decodedLog.args.outputs.map((output) => ({
							token: output.token,
							amount: output.amount,
							beneficiary: output.beneficiary,
						})),
						inputs: decodedLog.args.inputs.map((input) => ({
							token: input.token,
							amount: input.amount,
						})),
						callData: decodedLog.args.callData,
						transactionHash: decodedLog.transactionHash,
					}),
					user: decodedLog.args.user,
					sourceChain: hexToString(decodedLog.args.sourceChain),
					destChain: hexToString(decodedLog.args.destChain),
					deadline: decodedLog.args.deadline,
					nonce: decodedLog.args.nonce,
					fees: decodedLog.args.fees,
					outputs: decodedLog.args.outputs.map((output) => ({
						token: output.token,
						amount: output.amount,
						beneficiary: output.beneficiary,
					})),
					inputs: decodedLog.args.inputs.map((input) => ({
						token: input.token,
						amount: input.amount,
					})),
					callData: decodedLog.args.callData,
					transactionHash: decodedLog.transactionHash,
				}

				this.logger.info({ orderId: order.id, txHash: order.transactionHash }, "New order detected")
				this.emit("newOrder", { order })
			} catch (error) {
				this.logger.error({ err: error, log }, "Error parsing event log")
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
