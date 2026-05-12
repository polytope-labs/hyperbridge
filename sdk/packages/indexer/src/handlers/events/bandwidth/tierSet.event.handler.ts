import { SubstrateEvent } from "@subql/types"
import { Balance } from "@polkadot/types/interfaces"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { BandwidthService } from "@/services/bandwidth.service"

/**
 * `pallet-bandwidth :: TierSet` — admin set or revoked a tier SKU.
 *
 * Payload order:
 *   0. tier:   TierIndex
 *   1. config: Option<TierConfig { bytes: u128, duration_secs: u64 }>
 */
export const handleTierSetEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
	} = event
	logger.info(`Saw Bandwidth.TierSet on ${getHostStateMachine(chainId)}`)

	const tier = (data[0] as unknown as { toNumber: () => number }).toNumber()
	const cfgJson = data[1].toJSON() as { bytes: string | number; durationSecs: string | number } | null

	const blockTimestampMs = await getBlockTimestamp(
		block.block.header.hash.toString(),
		getHostStateMachine(chainId),
	)

	if (cfgJson) {
		await BandwidthService.applyTierSet({
			tier,
			bytes: BigInt(cfgJson.bytes),
			durationSecs: BigInt(cfgJson.durationSecs),
			blockTimestampMs,
		})
	} else {
		await BandwidthService.applyTierSet({ tier, blockTimestampMs })
	}
})
