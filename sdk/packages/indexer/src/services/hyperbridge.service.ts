import { PostRequestEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { EthereumTransaction } from "@subql/types-ethereum"
import { RelayerV2, Transfer } from "@/configs/src/types/models"
import { HyperBridgeStatsService } from "@/services/hyperbridgeStats.service"
import stringify from "safe-stable-stringify"
import { RelayerService } from "./relayer.service"

export class HyperBridgeService {
	/**
	 * Perform the necessary actions related to Hyperbridge stats when a PostRequest event is indexed
	 */
	static async handlePostRequestEvent(
		chain: string,
		event: PostRequestEventLog,
	): Promise<void> {
		if (!event.args) return

		logger.info(`handlePostRequestEvent: ${stringify({ chain, event })}`)

		try {
			await this.incrementNumberOfSentMessages(chain)
		} catch (error) {
			logger.error(
				`Error updating Hyperbridge stats related to PostRequest event: ${JSON.stringify({
					error,
					address: event.address,
				})}`,
			)
			return
		}
	}

	/**
	 * Perform the necessary actions related to Hyperbridge stats when a request is delivered (PostRequest or GetRequest handled).
	 * @param transaction Optional Ethereum transaction for EVM chains.
	 */
	static async handleRequestHandledEvent(
		relayer_id: string,
		chain: string,
		timestamp: bigint,
		transaction?: EthereumTransaction,
	): Promise<void> {
		await this.incrementNumberOfDeliveredMessages(chain)
		await RelayerService.updateMessageDelivered(relayer_id, chain, timestamp, transaction)
	}

	/**
	 * Increment the total number of messages sent on hyperbridge
	 */
	static async incrementNumberOfSentMessages(chain: string): Promise<void> {
		logger.info(`Incrementing number of messages sent on hyperbridge`)
		// Update the specific chain stats
		let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfMessagesSent += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of successful messages handled by hyperbridge
	 */
	static async incrementNumberOfDeliveredMessages(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfDeliveredMessages += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of failed deliveries by hyperbridge
	 */
	static async incrementNumberOfFailedDeliveries(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfFailedDeliveries += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of timed-out messages handled by hyperbridge
	 */
	static async incrementNumberOfTimedOutMessagesSent(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfTimedOutMessages += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Handle transfers out of the host account, incrementing the fees payed out to relayers
	 */
	static async handleTransferOutOfHostAccounts(transfer: Transfer, chain: string): Promise<void> {
		let relayer = await RelayerV2.get(transfer.to)

		if (typeof relayer !== "undefined") {
			let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)

			chainStats.feesPayedOutToRelayers += BigInt(transfer.amount)

			await chainStats.save()
		}
	}

	/**
	 * Increment the total amount transferred to hyperbridge (relayer fees)
	 */
	static async updateTotalTransfersIn(transfer: Transfer, chain: string): Promise<void> {
		// Update the specific chain metrics
		let chainStats = await HyperBridgeStatsService.findOrCreateChainStats(chain)
		chainStats.totalTransfersIn += BigInt(transfer.amount)

		await chainStats.save()
	}
}
