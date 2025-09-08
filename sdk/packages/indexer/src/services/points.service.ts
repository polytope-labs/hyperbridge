import { Rewards as RewardPoints, RewardsActivityLog as RewardPointsActivityLog } from "@/configs/src/types"
import { ProtocolParticipantType, PointsActivityType } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

export class PointsService {
	private static async getOrCreate(
		address: string,
		chain: string,
		earnerType: ProtocolParticipantType,
	): Promise<RewardPoints> {
		const rewardPointsId = `${address}-${chain}-${earnerType}`
		let rewardPoints = await RewardPoints.get(rewardPointsId)

		if (!rewardPoints) {
			logger.info(`Creating Reward Points for ${address} on ${chain} with earner type ${earnerType}`)
			rewardPoints = await RewardPoints.create({
				id: rewardPointsId,
				address,
				chain,
				points: BigInt(0),
				earnerType,
			})
		}

		await rewardPoints.save()

		logger.info(`Reward Points for ${address} on ${chain} with earner type ${earnerType} saved`)

		return rewardPoints
	}

	static async awardPoints(
		address: string,
		chain: string,
		points: bigint,
		earnerType: ProtocolParticipantType,
		activityType: PointsActivityType,
		transactionHash: string,
		description: string,
		timestamp: bigint,
	): Promise<void> {
		const rewardPoints = await this.getOrCreate(address, chain, earnerType)

		logger.info(`Awarding ${points} points to ${address} on ${chain} with earner type ${earnerType}`)

		rewardPoints.points = rewardPoints.points + points
		await rewardPoints.save()

		const activityLog = await RewardPointsActivityLog.create({
			id: `${address}-${earnerType}-${transactionHash}`,
			chain,
			points,
			transactionHash,
			earnerAddress: address,
			earnerType,
			activityType,
			description,
			createdAt: timestampToDate(timestamp),
		})
		await activityLog.save()
	}

	static async deductPoints(
		address: string,
		chain: string,
		points: bigint,
		earnerType: ProtocolParticipantType,
		activityType: PointsActivityType,
		transactionHash: string,
		description: string,
		timestamp: bigint,
	): Promise<void> {
		const rewardPoints = await this.getOrCreate(address, chain, earnerType)

		logger.info(`Deducting ${points} points from ${address} on ${chain} with earner type ${earnerType}`)

		rewardPoints.points = rewardPoints.points - points
		await rewardPoints.save()

		const activityLog = await RewardPointsActivityLog.create({
			id: `${address}-${earnerType}-${transactionHash}`,
			chain,
			points: -points, // Store as negative in activity log
			transactionHash,
			earnerAddress: address,
			earnerType,
			activityType,
			description,
			createdAt: timestampToDate(timestamp),
		})
		await activityLog.save()
	}

	static async getPoints(address: string, chain: string, earnerType: ProtocolParticipantType): Promise<bigint> {
		const rewardPoints = await RewardPoints.get(`${address}-${chain}-${earnerType}`)
		return rewardPoints?.points || BigInt(0)
	}

	/**
	 * Get reward points by address
	 */
	static async getByAddress(address: string) {
		return RewardPoints.getByAddress(address, {
			orderBy: "points",
			limit: -1,
		})
	}

	/**
	 * Get reward points by chain
	 */
	static async getByChain(chain: string) {
		return RewardPoints.getByChain(chain, {
			orderBy: "points",
			limit: -1,
		})
	}

	/**
	 * Get reward points by points amount
	 */
	static async getByPoints(points: bigint) {
		return RewardPoints.getByPoints(points, {
			orderBy: "points",
			limit: -1,
		})
	}
}
