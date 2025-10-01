import { SubstrateEvent } from "@subql/types"
import { Treasury } from "@/configs/src/types"
import { wrap } from "@/utils/event.utils"
import { Balance } from "@polkadot/types/interfaces"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { DailyTreasuryRewardService, TREASURY_ADDRESS } from "@/services/dailyTreasuryReward.service"

export const handleTreasuryTransferEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	try {
		const {
			event: { data },
			block,
		} = event

		const fromAddress = data[0].toString()
		const toAddress = data[1].toString()

		if (fromAddress !== TREASURY_ADDRESS && toAddress !== TREASURY_ADDRESS) {
			return
		}

		const amount = (data[2] as unknown as Balance).toBigInt()

		let treasury = await Treasury.get(TREASURY_ADDRESS)
		if (!treasury) {
			treasury = Treasury.create({
				id: TREASURY_ADDRESS,
				totalAmountTransferredIn: BigInt(0),
				totalAmountTransferredOut: BigInt(0),
				totalBalance: BigInt(0),
				lastUpdatedAt: new Date(block.timestamp!),
			})
		}

		if (fromAddress === TREASURY_ADDRESS) {
			treasury.totalAmountTransferredOut += amount
		} else {
			treasury.totalAmountTransferredIn += amount
		}

		treasury.totalBalance = await DailyTreasuryRewardService.getTreasuryBalance()
		const hyperbridgeChain = getHostStateMachine(chainId)
		const timestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)
		treasury.lastUpdatedAt = timestampToDate(timestamp)

		await treasury.save()
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle treasury transfer event: ${errorMessage}`)
	}
})
