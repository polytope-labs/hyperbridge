import { RewardPoints, RewardPointsActivityLog } from "@/configs/src/types"
import { ProtocolParticipant, RewardPointsActivityType } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

export class PointsService {
	private static async getOrCreate(
		address: string,
		chain: string,
		earnerType: ProtocolParticipant,
	): Promise<RewardPoints> {
		const rewardPointsId = `${address}-${chain}-${earnerType}`
		let rewardPoints = await RewardPoints.get(rewardPointsId)

		if (!rewardPoints) {
			rewardPoints = await RewardPoints.create({
				id: rewardPointsId,
				address,
				chain,
				points: BigInt(0),
				earnerType,
			})
		}

		return rewardPoints
	}

	static async awardPoints(
		address: string,
		chain: string,
		points: bigint,
		earnerType: ProtocolParticipant,
		activityType: RewardPointsActivityType,
		transactionHash: string,
		description: string,
		timestamp: bigint,
	): Promise<void> {
		const rewardPoints = await this.getOrCreate(address, chain, earnerType)

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
		earnerType: ProtocolParticipant,
		activityType: RewardPointsActivityType,
		transactionHash: string,
		description: string,
		timestamp: bigint,
	): Promise<void> {
		const rewardPoints = await this.getOrCreate(address, chain, earnerType)

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

	static async getPoints(address: string, chain: string, earnerType: ProtocolParticipant): Promise<bigint> {
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
