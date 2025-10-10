import { SubstrateEvent } from "@subql/types"
import { AccumulatedFee } from "@/configs/src/types"
import { DailyTreasuryRewardService } from "@/services/dailyTreasuryReward.service"
import { RelayerService } from "@/services/relayer.service"
import { encodeAddress } from "@polkadot/util-crypto"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"


export const handleAccumulateFeesEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	try {
		logger.info(`iN ACCumulated fee event`)

		const {
			event: { data },
			block,
		} = event

		const [relayerBytes, stateMachine, rawAmountCodec] = data

		logger.info(
			`accumulated fees event ${data}  ${block.block.header.number} gotten for relayer ${relayerBytes} on chain ${stateMachine} with amount ${rawAmountCodec}`,
		)

		const relayerAddress = encodeAddress(relayerBytes.toHex())
		const rawAmount = (rawAmountCodec as any).toBigInt()

		const stateMachineId = formatChain(stateMachine.toString())

		logger.info(
			`accumulated fees event gotten for relayer ${relayerBytes} on chain ${stateMachineId} with amount ${rawAmountCodec}`,
		)

		const recordId = `${relayerAddress}-${stateMachineId}`

		const hyperbridgeChain = getHostStateMachine(chainId)
		const timestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)
		const date = timestampToDate(timestamp)

		let record = await AccumulatedFee.get(recordId)
		if (!record) {
			record = AccumulatedFee.create({
				id: recordId,
				relayer: relayerAddress,
				chainId: stateMachineId,
				lifetimeFees: BigInt(0),
				lastUpdatedAt: date,
			})
		}

		const decimals = await DailyTreasuryRewardService.getFeeTokenDecimals(stateMachineId)
		logger.info(`accumulated fees event gotten for relayer ${relayerBytes}, with token fee decimals ${decimals}`)

		const normalizedAmount = rawAmount / 10n ** (18n - BigInt(decimals))

		record.lifetimeFees += normalizedAmount
		record.lastUpdatedAt = date

		await RelayerService.updateFeesEarnedViaAccumulation(
			relayerAddress,
			normalizedAmount,
			stateMachineId,
			timestamp,
		)

		await record.save()

		logger.info(`Updated accumulated fees for relayer ${relayerAddress} on chain ${stateMachineId}`)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to update accumulated fees: ${errorMessage}`)
	}
})
