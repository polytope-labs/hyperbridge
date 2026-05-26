import { PostRequestEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { EthereumTransaction } from "@subql/types-ethereum"
import { DailyProtocolFeesStats, RelayerV2, Transfer } from "@/configs/src/types/models"
import { HyperBridgeChainStatsService } from "@/services/hyperbridgeChainStats.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { getDateFormatFromTimestamp, isWithin24Hours, timestampToDate } from "@/utils/date.helpers"
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

		const { args, address } = event
		let { body, dest } = args

		logger.info(`handlePostRequestEvent: ${stringify({ chain, event })}`)

		try {
			const protocolFee = await this.computeProtocolFeeFromHexData(address, body, dest)
			await this.updateDailyProtocolFees(event.blockHash, protocolFee, chain)
			await this.incrementProtocolFeesEarned(protocolFee, chain)
			await this.incrementNumberOfSentMessages(chain)
		} catch (error) {
			logger.error(
				`Error updating Hyperbridge stats related to PostRequest event: ${JSON.stringify({
					error,
					address,
					body,
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
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfMessagesSent += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of successful messages handled by hyperbridge
	 */
	static async incrementNumberOfDeliveredMessages(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfDeliveredMessages += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of failed deliveries by hyperbridge
	 */
	static async incrementNumberOfFailedDeliveries(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfFailedDeliveries += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the number of timed-out messages handled by hyperbridge
	 */
	static async incrementNumberOfTimedOutMessagesSent(chain: string): Promise<void> {
		// Update the specific chain stats
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.numberOfTimedOutMessages += BigInt(1)

		await chainStats.save()
	}

	/**
	 * Increment the protocol fees earned by hyperbridge
	 */
	static async incrementProtocolFeesEarned(amount: bigint, chain: string): Promise<void> {
		logger.info(`Incrementing protocol fees earned by ${amount} on chain ${chain}`)
		// Update the specific chain stats
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.protocolFeesEarned += amount

		await chainStats.save()
	}

	/**
	 * Handle transfers out of the host account, incrementing the fees payed out to relayers
	 */
	static async handleTransferOutOfHostAccounts(transfer: Transfer, chain: string): Promise<void> {
		let relayer = await RelayerV2.get(transfer.to)

		if (typeof relayer !== "undefined") {
			let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)

			chainStats.feesPayedOutToRelayers += BigInt(transfer.amount)

			await chainStats.save()
		}
	}

	/**
	 * Increment the total amount transferred to hyperbridge (protocol fees + relayer fees)
	 */
	static async updateTotalTransfersIn(transfer: Transfer, chain: string): Promise<void> {
		// Update the specific chain metrics
		let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(chain)
		chainStats.totalTransfersIn += BigInt(transfer.amount)

		await chainStats.save()
	}

	/**
	 * Per-dispatch protocol fees on EVM hosts have been removed — bandwidth is
	 * pre-paid via `BandwidthManager.purchase()` and metered by `pallet-bandwidth`
	 * on Hyperbridge. Returns 0 so the daily-protocol-fees stat shape is preserved
	 * until indexing is rewired against bandwidth purchase events.
	 */
	static async computeProtocolFeeFromHexData(
		_contract_address: string,
		_data: string,
		_stateId: string,
	): Promise<bigint> {
		return 0n
	}

	static async updateDailyProtocolFees(blockHash: string, protocolFeeAmount: bigint, chain: string): Promise<void> {
		const stateMachineId = chain

		try {
			const timestamp = await getBlockTimestamp(blockHash, chain)

			const dateString = getDateFormatFromTimestamp(timestamp)
			const id = `${stateMachineId}.${dateString}`

			let dailyProtocolFees = await DailyProtocolFeesStats.get(id)

			if (!dailyProtocolFees) {
				dailyProtocolFees = DailyProtocolFeesStats.create({
					id,
					chain,
					stateMachineId,
					last24HoursProtocolFeesEarned: protocolFeeAmount,
					lastUpdatedAt: timestamp,
					createdAt: timestampToDate(timestamp),
				})
			}

			if (
				isWithin24Hours(dailyProtocolFees.createdAt, timestamp) &&
				dailyProtocolFees.lastUpdatedAt !== timestamp
			) {
				dailyProtocolFees.last24HoursProtocolFeesEarned += protocolFeeAmount
				dailyProtocolFees.lastUpdatedAt = timestamp
			}

			await dailyProtocolFees.save()
		} catch (error) {
			logger.error(`Error updating daily protocol fees for stateMachine: ${stateMachineId}`, error)
		}
	}
}
