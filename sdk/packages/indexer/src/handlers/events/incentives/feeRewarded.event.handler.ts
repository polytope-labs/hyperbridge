import { SubstrateEvent } from "@subql/types"
import { HyperbridgeRelayerReward } from "@/configs/src/types"
import { Balance } from "@polkadot/types/interfaces"
import { DailyTreasuryRewardService } from "@/services/dailyTreasuryReward.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export async function handleFeeRewardedEvent(event: SubstrateEvent): Promise<void> {
	try {
		const {
			event: { data },
			block,
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
		record.reputationAssetBalance = await DailyTreasuryRewardService.getReputationAssetBalance(relayerAddress)

		await record.save()

		const hyperbridgeChain = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)

		await DailyTreasuryRewardService.update(blockTimestamp, rewardAmount)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle fee rewarded event: ${errorMessage}`)
	}
}
