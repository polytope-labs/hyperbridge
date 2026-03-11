import { SubstrateEvent } from "@subql/types"
import { HyperbridgeRelayerReward, RelayerRewardTransaction, RelayerRewardType } from "@/configs/src/types"
import { Balance } from "@polkadot/types/interfaces"
import { DailyTreasuryRewardService } from "@/services/dailyTreasuryReward.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"

export async function handleFeeRewardedEvent(event: SubstrateEvent): Promise<void> {
	try {
		const {
			event: { data },
			block,
			extrinsic,
		} = event

		const [relayer, amount] = data
		const relayerAddress = relayer.toString()
		const rewardAmount = (amount as unknown as Balance).toBigInt()

		let record = await HyperbridgeRelayerReward.get(relayerAddress)
		if (!record) {
			record = HyperbridgeRelayerReward.create({
				id: relayerAddress,
			})
		}

		record.totalMessagingRewardAmount = (record.totalMessagingRewardAmount ?? BigInt(0)) + rewardAmount
		record.totalRewardAmount = (record.totalRewardAmount ?? BigInt(0)) + rewardAmount

		await record.save()

		const hyperbridgeChain = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)

		await DailyTreasuryRewardService.update(blockTimestamp, rewardAmount)

		const blockNumber = block.block.header.number.toBigInt()
		const extrinsicIndex = extrinsic?.idx ?? 0
		const transactionId = `${relayerAddress}-${blockNumber}-${extrinsicIndex}`

		const rewardTransaction = RelayerRewardTransaction.create({
			id: transactionId,
			relayer: relayerAddress,
			chain: hyperbridgeChain,
			amount: rewardAmount,
			rewardType: RelayerRewardType.MESSAGING_REWARD,
			blockNumber,
			blockTimestamp,
			extrinsicHash: extrinsic?.extrinsic.hash.toString(),
			createdAt: timestampToDate(blockTimestamp),
		})
		await rewardTransaction.save()

		logger.info(`Created fee reward transaction: ${transactionId}`)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle fee rewarded event: ${errorMessage}`)
	}
}
