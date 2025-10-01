import { SubstrateEvent } from "@subql/types"
import { HyperbridgeRelayerReward } from "@/configs/src/types"
import { wrap } from "@/utils/event.utils"
import { Balance } from "@polkadot/types/interfaces"
import { DailyTreasuryRewardService } from "@/services/dailyTreasuryReward.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export const handleRelayerRewardedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	try {
		const {
			event: { data, method },
			block,
		} = event
		logger.info(`Relayer Rewarded Event ${method} event at block: ${block.block.header.number.toString()}`)

		const [relayer, amount, stateMachineHeight] = data

		const relayerAddress = relayer.toString()
		const rewardAmount = (amount as unknown as Balance).toBigInt()

		let record = await HyperbridgeRelayerReward.get(relayerAddress)
		if (!record) {
			record = HyperbridgeRelayerReward.create({
				id: relayerAddress,
			})
		}

		logger.info(`Saving Relayer Rewarded Event ${method} event at block: ${record}`)

		record.totalConsensusRewardAmount = (record.totalConsensusRewardAmount ?? BigInt(0)) + rewardAmount
		record.totalRewardAmount = (record.totalRewardAmount ?? BigInt(0)) + rewardAmount
		record.reputationAssetBalance = await DailyTreasuryRewardService.getReputationAssetBalance(relayerAddress)

		await record.save()

		const hyperbridgeChain = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)
		await DailyTreasuryRewardService.update(blockTimestamp, rewardAmount)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle relayer rewarded event: ${errorMessage}`)
	}
})
