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
import { Mutex } from "async-mutex"

export class EventMonitor extends EventEmitter {
	private chains: Map<number, IEvmChain> = new Map()
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
				const intentGatewayAddress = this.configService.getIntentGatewayV2Address(`EVM-${chainId}`)

				const client = chain.client
				if (!client) {
					throw new Error(`Chain ${chainId} does not expose a public client`)
				}

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
							await this.scanBlocks(chainId, chain, intentGatewayAddress, gatewayEvents)
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
		intentGatewayAddress: `0x${string}`,
		gatewayEvents: any[],
	): Promise<void> {
		const lastScanned = this.lastScannedBlock.get(chainId)
		if (!lastScanned) return

		const currentBlock = await retryPromise(() => chain.client.getBlockNumber(), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: "Failed to get current block number",
		})

		if (currentBlock > lastScanned) {
			const fromBlock = lastScanned + 1n

			const maxBlockRange = 1000n
			const clampedToBlock =
				fromBlock + maxBlockRange > currentBlock ? currentBlock : fromBlock + maxBlockRange

			this.logger.debug(
				{ chainId, fromBlock, toBlock: clampedToBlock, gap: Number(clampedToBlock - fromBlock) },
				"Scanning blocks",
			)

			let logs: any[]
			let actualToBlock = clampedToBlock

			try {
				logs = await retryPromise(
					() =>
						chain.client.getLogs({
							address: intentGatewayAddress,
							events: gatewayEvents,
							fromBlock,
							toBlock: clampedToBlock,
						}),
					{
						maxRetries: 3,
						backoffMs: 250,
						logMessage: "Failed to get gateway event logs",
					},
				)
			} catch (error: any) {
				if (!this.isBlockRangeError(error)) throw error

				// RPC node is behind the reported head block — fall back to "latest" tag
				this.logger.warn(
					{ chainId, fromBlock, toBlock: clampedToBlock },
					"Block range ahead of RPC head, retrying with 'latest'",
				)

				try {
					logs = await chain.client.getLogs({
						address: intentGatewayAddress,
						events: gatewayEvents,
						fromBlock,
					})

					// Determine actual toBlock from log results or re-fetch head
					if (logs.length > 0) {
						actualToBlock = logs.reduce(
							(max: bigint, log: any) =>
								log.blockNumber > max ? log.blockNumber : max,
							fromBlock,
						)
					} else {
						const latestBlock = await chain.client.getBlockNumber()
						actualToBlock = latestBlock >= fromBlock ? latestBlock : fromBlock
					}
				} catch (fallbackError) {
					this.logger.warn(
						{ chainId, fromBlock, err: fallbackError },
						"Fallback scan with 'latest' also failed, will retry next interval",
					)
					return
				}
			}

			const placedLogs = logs.filter((l: any) => l.eventName === "OrderPlaced")
			const filledLogs = logs.filter(
				(l: any) => l.eventName === "OrderFilled" || l.eventName === "PartialFill",
			)

			if (placedLogs.length > 0) {
				this.logger.info(
					{ chainId, fromBlock, toBlock: actualToBlock, eventCount: placedLogs.length },
					"Found OrderPlaced events in block scan",
				)
				await this.processOrderPlacedLogs(chainId, chain, placedLogs)
			}

			if (filledLogs.length > 0) {
				this.logger.info(
					{ chainId, fromBlock, toBlock: actualToBlock, eventCount: filledLogs.length },
					"Found OrderFilled events in block scan",
				)
				this.processOrderFilledLogs(chainId, filledLogs)
			}

			// Update lastScannedBlock only after successful processing
			// This is protected by the mutex, so no race condition
			this.lastScannedBlock.set(chainId, actualToBlock)
		}
	}

	private async processOrderPlacedLogs(chainId: number, chain: IEvmChain, logs: any[]): Promise<void> {
		for (const log of logs) {
			try {
				const decodedLog = log as unknown as DecodedOrderPlacedLog
				const transactionHash = decodedLog.transactionHash
				let order: Order = {
					user: decodedLog.args.user,
					source: hexToString(decodedLog.args.source) as HexString,
					destination: hexToString(decodedLog.args.destination) as HexString,
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
				}

				const intentGatewayAddress = this.configService.getIntentGatewayV2Address(order.source)
				const placeOrderCallInput = await chain.getPlaceOrderCalldata!(
					transactionHash as string,
					intentGatewayAddress,
				)

				const decodedCalldata = decodeFunctionData({
					abi: INTENT_GATEWAY_V2_ABI,
					data: placeOrderCallInput as HexString,
				})?.args?.[0] as Order

				order.output.beneficiary = decodedCalldata.output.beneficiary as `0x${string}`
				order.output.call = decodedCalldata.output.call as HexString
				order.predispatch.call = decodedCalldata.predispatch.call as HexString
				order.id = orderCommitment(order)

				this.logger.info({ orderId: order.id, txHash: transactionHash }, "New order detected")
				this.emit("newOrder", { order, transactionHash })
			} catch (error) {
				this.logger.error({ err: error, log }, "Error parsing event log")
			}
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
