import { HyperBridgeChainStats } from "../../configs/src/types"

export class HyperBridgeChainStatsService {
	/**
	 * Find the HyperBridgeChainStats record for a chain, create it if it doesn't exist
	 */
	static async findOrCreateChainStats(chain: string): Promise<HyperBridgeChainStats> {
		let chainStats = await HyperBridgeChainStats.get(chain)

		if (typeof chainStats === "undefined") {
			chainStats = HyperBridgeChainStats.create({
				id: chain,
				totalTransfersIn: BigInt(0),
				protocolFeesEarned: BigInt(0),
				feesPayedOutToRelayers: BigInt(0),
				numberOfMessagesSent: BigInt(0),
				numberOfDeliveredMessages: BigInt(0),
				numberOfFailedDeliveries: BigInt(0),
				numberOfTimedOutMessages: BigInt(0),
			})
			await chainStats.save()
		}

		return chainStats
	}

	/**
	 * Get chains by number of messages sent
	 */
	static async getByNumberOfMessagesSent(numberOfMessagesSent: bigint) {
		return HyperBridgeChainStats.getByNumberOfMessagesSent(numberOfMessagesSent, {
			orderBy: "numberOfMessagesSent",
			limit: -1,
		})
	}

	/**
	 * Get chains by number of delivered messages
	 */
	static async getByNumberOfDeliveredMessages(numberOfDeliveredMessages: bigint) {
		return HyperBridgeChainStats.getByNumberOfDeliveredMessages(numberOfDeliveredMessages, {
			orderBy: "numberOfDeliveredMessages",
			limit: -1,
		})
	}

	/**
	 * Get chains by number of failed deliveries
	 */
	static async getByNumberOfFailedDeliveries(numberOfFailedDeliveries: bigint) {
		return HyperBridgeChainStats.getByNumberOfFailedDeliveries(numberOfFailedDeliveries, {
			orderBy: "numberOfFailedDeliveries",
			limit: -1,
		})
	}

	/**
	 * Get chains by total transfers in
	 */
	static async getByTotalTransfersIn(totalTransfersIn: bigint) {
		return HyperBridgeChainStats.getByTotalTransfersIn(totalTransfersIn, {
			orderBy: "totalTransfersIn",
			limit: -1,
		})
	}

	/**
	 * Get chains by protocol fees earned
	 */
	static async getByProtocolFeesEarned(protocolFeesEarned: bigint) {
		return HyperBridgeChainStats.getByProtocolFeesEarned(protocolFeesEarned, {
			orderBy: "protocolFeesEarned",
			limit: -1,
		})
	}

	/**
	 * Get chains by fees payed out to relayers
	 */
	static async getByFeesPayedOutToRelayers(feesPayedOutToRelayers: bigint) {
		return HyperBridgeChainStats.getByFeesPayedOutToRelayers(feesPayedOutToRelayers, {
			orderBy: "feesPayedOutToRelayers",
			limit: -1,
		})
	}
}
