import { RelayerV2, RelayerActivity, Transfer } from "@/configs/src/types/models"
import { RelayerStatsPerChainV2Service } from "@/services/relayerChainStats.service"
import { EthereumTransaction } from "@subql/types-ethereum"
import PriceHelper from "@/utils/price.helpers"
import { PointsService } from "@/services/points.service"
import { PointsActivityType, ProtocolParticipantType } from "@/configs/src/types"
import { GET_ETHEREUM_L2_STATE_MACHINES } from "@/utils/l2-state-machine.helper"

export class RelayerService {
	/**
	 * Find a relayer by its id or create a new one if it doesn't exist
	 */
	static async findOrCreate(relayer_id: string, chain: string, timestamp: bigint): Promise<RelayerV2> {
		let relayer = await RelayerV2.get(relayer_id)

		if (typeof relayer === "undefined") {
			relayer = RelayerV2.create({ id: relayer_id })
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
		const relayer_chain_stats = await RelayerStatsPerChainV2Service.findOrCreate(relayer.id, transfer.chain)

		relayer_chain_stats.feesEarned += transfer.amount
		await this.updateRelayerActivity(relayer.id, timestamp)

		await relayer.save()
		await relayer_chain_stats.save()
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
		const relayer_chain_stats = await RelayerStatsPerChainV2Service.findOrCreate(relayer.id, chain)

		relayer_chain_stats.feesEarned += fee
		await this.updateRelayerActivity(relayer.id, timestamp)

		await relayer.save()
		await relayer_chain_stats.save()
	}

	/**
	 * Update message delivered by the relayer with gas cost tracking
	 * @param relayer_id The relayer address
	 * @param chain The chain identifier
	 * @param timestamp The block timestamp
	 * @param transaction Optional Ethereum transaction for EVM chains. When provided, gas costs are tracked.
	 */
	static async updateMessageDelivered(
		relayer_id: string,
		chain: string,
		timestamp: bigint,
		transaction?: EthereumTransaction,
	): Promise<void> {
		const relayer = await this.findOrCreate(relayer_id, chain, timestamp)
		const relayer_chain_stats = await RelayerStatsPerChainV2Service.findOrCreate(relayer.id, chain)

		if (transaction) {
			const receipt = await transaction.receipt()
			const { gasUsed, effectiveGasPrice } = receipt

			const nativeCurrencyPrice = await PriceHelper.getNativeCurrencyPrice(chain)
			let gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed)

			// Add L1 fee for L2 chains
			if (GET_ETHEREUM_L2_STATE_MACHINES().includes(chain)) {
				const l1Fee = BigInt((receipt as any).l1Fee ?? 0)
				gasFee += l1Fee
			}

			const usdFee = (gasFee * nativeCurrencyPrice) / 10n ** 18n

			relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1)
			relayer_chain_stats.gasUsedForSuccessfulMessages += BigInt(gasUsed)
			relayer_chain_stats.gasFeeForSuccessfulMessages += gasFee
			relayer_chain_stats.usdGasFeeForSuccessfulMessages += usdFee

			await PointsService.awardPoints(
				relayer_id,
				chain,
				BigInt(50),
				ProtocolParticipantType.RELAYER,
				PointsActivityType.REWARD_POINTS_EARNED,
				transaction.hash,
				"Points awarded for successful message delivered",
				timestamp,
			)
		} else {
			// For non evm chains without transaction
			relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1)
		}

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

}
