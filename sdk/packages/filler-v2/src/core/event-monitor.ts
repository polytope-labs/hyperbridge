import { EventEmitter } from "events"
import {
	ChainConfig,
	OrderV2,
	orderV2Commitment,
	hexToString,
	retryPromise,
	DecodedOrderV2PlacedLog,
	getContractCallInput,
	HexString,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { PublicClient, decodeFunctionData } from "viem"
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
				const orderPlacedEvent = INTENT_GATEWAY_V2_ABI.find(
					(item) => item.type === "event" && item.name === "OrderPlaced",
				)
				const intentGatewayAddress = this.configService.getIntentGatewayV2Address(`EVM-${chainId}`)

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
				await this.processLogs(client, logs)
			}

			// Update lastScannedBlock only after successful processing
			// This is protected by the mutex, so no race condition
			this.lastScannedBlock.set(chainId, actualToBlock)
		}
	}

	private async processLogs(client: PublicClient, logs: any[]): Promise<void> {
		for (const log of logs) {
			try {
				const decodedLog = log as unknown as DecodedOrderV2PlacedLog
				let order: OrderV2 = {
					id: "",
					user: decodedLog.args.user,
					source: hexToString(decodedLog.args.source),
					destination: hexToString(decodedLog.args.destination),
					deadline: decodedLog.args.deadline,
					nonce: decodedLog.args.nonce,
					fees: decodedLog.args.fees,
					session: decodedLog.args.session,
					predispatch: {
						assets: decodedLog.args.predispatch.map((predispatch) => ({
							token: predispatch.token,
							amount: predispatch.amount,
						})),
						call: "0x",
					},
					output: {
						beneficiary: "0x0000000000000000000000000000000000000000",
						assets: decodedLog.args.outputs.map((output) => ({
							token: output.token,
							amount: output.amount,
						})),
						call: "0x",
					},
					inputs: decodedLog.args.inputs.map((input) => ({
						token: input.token,
						amount: input.amount,
					})),
					transactionHash: decodedLog.transactionHash,
				}

				// Get the other missing data using callTracer and calculate commitment
				const intentGatewayAddress = this.configService.getIntentGatewayV2Address(order.source)

				const placeOrderCallInput = await getContractCallInput(
					client as any,
					order.transactionHash as HexString,
					intentGatewayAddress,
				)

				const decodedCalldata = decodeFunctionData({
					abi: INTENT_GATEWAY_V2_ABI,
					data: placeOrderCallInput as HexString,
				})?.args?.[0] as OrderV2

				order.output.beneficiary = decodedCalldata.output.beneficiary as `0x${string}`
				order.output.call = decodedCalldata.output.call as HexString
				order.predispatch.call = decodedCalldata.predispatch.call as HexString
				order.id = orderV2Commitment(order)

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
