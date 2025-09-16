import { EventEmitter } from "events"
import {
	ChainConfig,
	Order,
	orderCommitment,
	DUMMY_PRIVATE_KEY,
	hexToString,
	DecodedOrderPlacedLog,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { PublicClient } from "viem"
import { ChainClientManager } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"

export class EventMonitor extends EventEmitter {
	private clients: Map<number, PublicClient> = new Map()
	private listening: boolean = false
	private unwatchFunctions: Map<number, () => void> = new Map()
	private clientManager: ChainClientManager
	private configService: FillerConfigService
	private logger = getLogger("event-monitor")

	constructor(chainConfigs: ChainConfig[], configService: FillerConfigService) {
		super()

		this.configService = configService
		this.clientManager = new ChainClientManager(configService)

		chainConfigs.forEach((config) => {
			const chainName = `EVM-${config.chainId}`
			const client = this.clientManager.getPublicClient(chainName)
			this.clients.set(config.chainId, client)
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
				const unwatch = client.watchEvent({
					address: intentGatewayAddress,
					event: orderPlacedEvent,
					onLogs: (logs) => {
						for (const log of logs) {
							try {
								const decodedLog = log as unknown as DecodedOrderPlacedLog
								const tempOrder: Order = {
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
								}

								const orderId = orderCommitment(tempOrder)

								const order: Order = {
									...tempOrder,
									id: orderId,
								}

								this.emit("newOrder", { order })
							} catch (error) {
								this.logger.error({ err: error }, "Error parsing event log")
							}
						}
					},
					poll: true,
					pollingInterval: 1000,
				})
				this.unwatchFunctions.set(chainId, unwatch)

				this.logger.info({ chainId }, "Started watching OrderPlaced events")
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Failed to create event filter")
			}
		}
	}

	public async stopListening(): Promise<void> {
		for (const [chainId, unwatch] of this.unwatchFunctions.entries()) {
			try {
				unwatch()
				this.logger.info({ chainId }, "Stopped watching for events")
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Error stopping event watcher")
			}
		}

		this.unwatchFunctions.clear()
		this.listening = false
	}
}
