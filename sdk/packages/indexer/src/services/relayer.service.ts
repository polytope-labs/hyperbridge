import { Relayer, RelayerActivity, Transfer } from "@/configs/src/types/models"
import { RelayerChainStatsService } from "@/services/relayerChainStats.service"
import {
	HandlePostRequestsTransaction,
	HandlePostResponsesTransaction,
} from "@/configs/src/types/abi-interfaces/HandlerV1Abi"
import PriceHelper from "@/utils/price.helpers"
import { PointsService } from "@/services/points.service"
import { PointsActivityType, ProtocolParticipantType } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { GET_ETHEREUM_L2_STATE_MACHINES } from "@/utils/l2-state-machine.helper"

export class RelayerService {
	/**
	 * Find a relayer by its id or create a new one if it doesn't exist
	 */
	static async findOrCreate(relayer_id: string, chain: string, timestamp: bigint): Promise<Relayer> {
		let relayer = await Relayer.get(relayer_id)

		if (typeof relayer === "undefined") {
			relayer = Relayer.create({ id: relayer_id })
			await relayer.save()
		}

		return relayer
	}

	/**
	 * Update the total fees earned by a relayer
	 * Fees earned by a relayer == Sum of all transfers to the relayer from the hyperbridge host address
	 */
	static async updateFeesEarned(transfer: Transfer, timestamp: bigint): Promise<void> {
		const relayer = await this.findOrCreate(transfer.to, transfer.chain, timestamp)
		const relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer.id, transfer.chain)

		relayer_chain_stats.feesEarned += transfer.amount
		await this.updateRelayerActivity(relayer.id, timestamp)

		relayer.save()
		relayer_chain_stats.save()
	}

	/**
	 * Update the total fees earned by a relayer via accumulation
	 */
	static async updateFeesEarnedViaAccumulation(
		relayer_id: string,
		fee: bigint,
		chain: any,
		timestamp: bigint,
	): Promise<void> {
		const relayer = await this.findOrCreate(relayer_id, chain, timestamp)
		const relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer.id, chain)

		relayer_chain_stats.feesEarned += fee
		await this.updateRelayerActivity(relayer.id, timestamp)

		relayer.save()
		relayer_chain_stats.save()
	}

	/**
	 * Update message delivered by the relayer
	 * @param relayer_id The relayer address
	 * @param chain The chain identifier
	 */
	static async updateMessageDelivered(relayer_id: string, chain: string, timestamp: bigint): Promise<void> {
		const relayer = await this.findOrCreate(relayer_id, chain, timestamp)
		const relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer.id, chain)

		relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1)
		await this.updateRelayerActivity(relayer.id, timestamp)

		await relayer.save()
		await relayer_chain_stats.save()
	}

	/**
	 * Update relayer activity
	 * @param relayerId The relayer address
	 * @param timestamp The timestamp of the activit
	 */
	static async updateRelayerActivity(relayerId: string, timestamp: bigint) {
		let activity = await RelayerActivity.get(relayerId)
		if (!activity) {
			activity = RelayerActivity.create({ id: relayerId, relayerId, lastUpdatedAt: timestamp })
		}

		activity.lastUpdatedAt = timestamp
		await activity.save()
	}

	/**
	 * Computes relayer specific stats from the handlePostRequest/handlePostResponse transactions on the handlerV1 contract
	 */
	static async handlePostRequestOrResponseTransaction(
		chain: string,
		transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction,
	): Promise<void> {
		const { from: relayer_id, hash: transaction_hash, blockHash } = transaction
		const receipt = await transaction.receipt()
		const { status, gasUsed, effectiveGasPrice } = receipt

		const nativeCurrencyPrice = await PriceHelper.getNativeCurrencyPrice(chain)

		let gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed)

		// Add the L1 Gas Used for L2 chains
		if (GET_ETHEREUM_L2_STATE_MACHINES().includes(chain)) {
			if ((receipt as any).l1Fee) {
				const l1Fee = BigInt((receipt as any).l1Fee ?? 0)
				gasFee += l1Fee
			} else {
				logger.error(
					`Could not find l1Fee in transaction receipt: ${JSON.stringify({
						chain,
						transactionHash: transaction.hash,
					})}`,
				)
			}
		}

		const usdFee = (gasFee * nativeCurrencyPrice) / (10n ** 18n);
		const gasFeeInEth = Number(gasFee) / 1e18;

		try {
			const timestamp = await getBlockTimestamp(blockHash, chain)

			let relayer = await RelayerService.findOrCreate(relayer_id, chain, timestamp)
			let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer_id, chain)

			let pointsToAWard = 50;
			let description = "`Points awarded for successful message delivered`";
			if (status === true) {
				relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1)
				relayer_chain_stats.gasUsedForSuccessfulMessages += BigInt(gasUsed)
				relayer_chain_stats.gasFeeForSuccessfulMessages += BigInt(gasFee)
				relayer_chain_stats.usdGasFeeForSuccessfulMessages += usdFee
			} else {
				relayer_chain_stats.numberOfFailedMessagesDelivered += BigInt(1)
				relayer_chain_stats.gasUsedForFailedMessages += BigInt(gasUsed)
				relayer_chain_stats.gasFeeForFailedMessages += BigInt(gasFee)
				relayer_chain_stats.usdGasFeeForFailedMessages += usdFee

				pointsToAWard = pointsToAWard / 2;
				description = "`Points awarded for failed message delivery`"
			}

			await PointsService.awardPoints(
				relayer_id,
				chain,
				BigInt(pointsToAWard),
				ProtocolParticipantType.RELAYER,
				PointsActivityType.REWARD_POINTS_EARNED,
				transaction_hash,
				description,
				timestamp,
			)


			await relayer.save()
			await relayer_chain_stats.save()

			logger.info(`Relayer: ${relayer_id} updated successfully for chain: ${chain}`)
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : String(e)
			logger.error(
				`Error while handling PostRequest/PostResponse transaction relayer updates: ${errorMessage}`,
			)
		}
	}
}
