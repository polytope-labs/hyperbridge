import { SubstrateEvent } from "@subql/types"
import { HyperbridgeCollatorReward, DailyTreasuryCollatorReward, HyperbridgeCollatorRewardTransaction } from "@/configs/src/types"
import { wrap } from "@/utils/event.utils"
import { Balance } from "@polkadot/types/interfaces"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"

/**
 * Handles CollatorRewarded events from pallet-collator-manager
 * Emitted when a collator is rewarded for authoring a block
 */
export const handleCollatorRewardedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	try {
		const {
			event: { data, method },
			block,
		} = event

		const [collator, amount] = data
		const collatorAddress = collator.toString()
		const rewardAmount = (amount as unknown as Balance).toBigInt()

		let record = await HyperbridgeCollatorReward.get(collatorAddress)
		if (!record) {
			record = HyperbridgeCollatorReward.create({
				id: collatorAddress,
				totalRewardAmount: BigInt(0),
				totalBlocksAuthored: BigInt(0),
			})
		}

		record.totalRewardAmount = (record.totalRewardAmount ?? BigInt(0)) + rewardAmount
		record.totalBlocksAuthored = (record.totalBlocksAuthored ?? BigInt(0)) + BigInt(1)
		record.lastRewardedAt = new Date(block.timestamp!)

		await record.save()

		const hyperbridgeChain = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(block.block.header.hash.toString(), hyperbridgeChain)
		await updateDailyCollatorReward(blockTimestamp, rewardAmount)

		const blockNumber = block.block.header.number.toBigInt()
		const transactionId = `${collatorAddress}-${blockNumber}`

		const rewardTransaction = HyperbridgeCollatorRewardTransaction.create({
			id: transactionId,
			collator: collatorAddress,
			amount: rewardAmount,
			blockNumber,
			blockTimestamp,
			createdAt: timestampToDate(blockTimestamp),
		})
		await rewardTransaction.save()

		logger.info(`Collator reward indexed: ${collatorAddress} received ${rewardAmount.toString()}, transaction: ${transactionId}`)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle collator reward event: ${errorMessage}`)
	}
})

/**
 * Updates the daily collator reward tracking
 */
async function updateDailyCollatorReward(timestamp: bigint, amount: bigint): Promise<void> {
	const day = timestampToDate(timestamp)
	day.setUTCHours(0, 0, 0, 0)
	const id = day.toISOString().slice(0, 10)

	let record = await DailyTreasuryCollatorReward.get(id)

	if (!record) {
		record = DailyTreasuryCollatorReward.create({
			id: id,
			date: day,
			dailyRewardAmount: BigInt(0),
		})
	}

	record.dailyRewardAmount += amount
	await record.save()
}
