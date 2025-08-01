import { PostRequestEventLog, PostResponseEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { DailyProtocolFeesStats, Relayer, Transfer } from "@/configs/src/types/models"
import { HyperBridgeChainStatsService } from "@/services/hyperbridgeChainStats.service"
import { isHexString } from "ethers/lib/utils"
import { EthereumHostAbi__factory } from "@/configs/src/types/contracts"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { getDateFormatFromTimestamp, isWithin24Hours, timestampToDate } from "@/utils/date.helpers"
import { RelayerService } from "./relayer.service"
// import {
//  HandlePostRequestsTransaction,
//  HandlePostResponsesTransaction,
// } from '@/types/abi-interfaces/HandlerV1Abi';

export class HyperBridgeService {
	/**
	 * Perform the necessary actions related to Hyperbridge stats when a PostRequest/PostResponse event is indexed
	 */
	static async handlePostRequestOrResponseEvent(
		chain: string,
		event: PostRequestEventLog | PostResponseEventLog,
	): Promise<void> {
		if (!event.args) return

		const { args, address } = event
		let { body, dest } = args

		logger.info(`handlePostRequestOrResponseEvent: ${stringify({ chain, event })}`)

		try {
			const protocolFee = await this.computeProtocolFeeFromHexData(address, body, dest)
			await this.updateDailyProtocolFees(event.blockHash, protocolFee, chain)
			await this.incrementProtocolFeesEarned(protocolFee, chain)
			await this.incrementNumberOfSentMessages(chain)
		} catch (error) {
			logger.error(
				`Error computing protocol fee: ${JSON.stringify({
					error,
					address,
					body,
				})}`,
			)
			return
		}
	}

	/**
	 * Perform the necessary actions related to Hyperbridge stats when a PostRequestHandled/PostResponseHandled event is indexed
	 */
	static async handlePostRequestOrResponseHandledEvent(
		relayer_id: string,
		chain: string,
		timestamp: bigint,
	): Promise<void> {
		await this.incrementNumberOfDeliveredMessages(chain)
		await RelayerService.updateMessageDelivered(relayer_id, chain, timestamp)
	}

	//  /**
	//   * Handle PostRequest or PostResponse transactions
	//   */
	//  static async handlePostRequestOrResponseTransaction(
	//   chain: string,
	//   transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction,
	//  ): Promise<void> {
	//   logger.info(
	//    `Creating PostRequest or PostResponse transaction update: ${JSON.stringify({
	//     transaction,
	//    })}`
	//   );
	//   const { status } = await transaction.receipt();

	//   if (status === false) {
	//    await this.incrementNumberOfFailedDeliveries(chain);
	//   }
	//  }

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
		let relayer = await Relayer.get(transfer.to)

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

	static async computeProtocolFeeFromHexData(
		contract_address: string,
		data: string,
		stateId: string,
	): Promise<bigint> {
		data = isHexString(data) ? data.slice(2) : data
		const noOfBytesInData = data.length / 2
		const evmHostContract = EthereumHostAbi__factory.connect(contract_address, api)
		logger.info(
			`Computing protocol fee for data: ${JSON.stringify({
				data,
				noOfBytesInData,
				stateId,
			})}`,
		)
		const encoder = new TextEncoder()
		const stateIdByte = encoder.encode(stateId)
		const perByteFee = await evmHostContract.perByteFee(stateIdByte)
		return perByteFee.mul(noOfBytesInData).toBigInt()
	}

	static async updateDailyProtocolFees(blockHash: string, protocolFeeAmount: bigint, chain: string): Promise<void> {
		const stateMachineId = getHostStateMachine(chain)

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
