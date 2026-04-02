import { RelayerStatsPerChainV2 } from "@/configs/src/types"

export class RelayerStatsPerChainV2Service {
	/*
	 * Find the RelayerStatsPerChainV2 record for a relayer on a chain, create it if it doesn't exist
	 */
	static async findOrCreate(relayer_id: string, chain: string): Promise<RelayerStatsPerChainV2> {
		let id = `${relayer_id}-${chain}`
		let metrics = await RelayerStatsPerChainV2.get(id)

		if (!metrics) {
			metrics = RelayerStatsPerChainV2.create({
				id,
				relayerId: relayer_id,
				chain,
				numberOfSuccessfulMessagesDelivered: BigInt(0),
				gasUsedForSuccessfulMessages: BigInt(0),
				gasFeeForSuccessfulMessages: BigInt(0),
				usdGasFeeForSuccessfulMessages: BigInt(0),
				feesEarned: BigInt(0),
				cumulativeWithdrawnAmount: BigInt(0),
			})
			await metrics.save()
		}

		return metrics
	}

	/**
	 * Get stats by fees earned
	 */
	static async getByFeesEarned(fees: bigint) {
		return RelayerStatsPerChainV2.getByFeesEarned(fees, {
			orderBy: "feesEarned",
			limit: -1,
		})
	}
}
