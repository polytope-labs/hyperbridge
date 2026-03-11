import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { encodeAddress } from "@polkadot/util-crypto"
import { hexToU8a } from "@polkadot/util"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { RelayerService } from "@/services/relayer.service"
import { RelayerStatsPerChainV2Service } from "@/services/relayerChainStats.service"

export const handleRelayerWithdrawEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	try {
		const {
			event: { data },
			block,
		} = event

		const [relayerBytes, _beneficiaryBytes, stateMachine, rawAmountCodec] = data

		logger.info(
			`Relayer withdraw event at block ${block.block.header.number} for relayer ${relayerBytes} on chain ${stateMachine} with amount ${rawAmountCodec}`,
		)

		const relayerHex = relayerBytes.toHex()
		const bytes = hexToU8a(relayerHex)

		const relayerAddress = bytes.length === 20 ? relayerHex : encodeAddress(relayerHex)
		const rawAmount = (rawAmountCodec as any).toBigInt()
		const stateMachineId = formatChain(stateMachine.toString())

		const hyperbridgeChain = getHostStateMachine(chainId)
		const timestamp = await getBlockTimestamp(event.block.block.header.hash.toString(), hyperbridgeChain)

		await RelayerService.findOrCreate(relayerAddress, stateMachineId, timestamp)
		const relayerChainStats = await RelayerStatsPerChainV2Service.findOrCreate(relayerAddress, stateMachineId)

		relayerChainStats.cumulativeWithdrawnAmount += rawAmount
		await relayerChainStats.save()

		logger.info(`Updated cumulative withdrawn amount for relayer ${relayerAddress} on chain ${stateMachineId}`)
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		logger.error(`Failed to handle relayer withdraw event: ${errorMessage}`)
	}
})
