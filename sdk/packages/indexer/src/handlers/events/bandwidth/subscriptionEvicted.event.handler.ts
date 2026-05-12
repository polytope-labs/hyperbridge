import { SubstrateEvent } from "@subql/types"
import { Balance } from "@polkadot/types/interfaces"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { BandwidthService } from "@/services/bandwidth.service"

/**
 * `pallet-bandwidth :: SubscriptionEvicted` — fired when a push onto a
 * full 1024-slot list drops the oldest subscription.
 *
 * Payload order:
 *   0. app_chain:  StateMachine
 *   1. app:        AppKey
 *   2. tier:       TierIndex
 *   3. lost_bytes: u128
 */
export const handleSubscriptionEvictedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
	} = event
	logger.info(`Saw Bandwidth.SubscriptionEvicted on ${getHostStateMachine(chainId)}`)

	const chain = formatChain(data[0].toJSON())
	const appHex = BandwidthService.normalizeAppHex(data[1].toHex())
	const tier = (data[2] as unknown as { toNumber: () => number }).toNumber()
	const lostBytes = (data[3] as unknown as Balance).toBigInt()

	const blockTimestampMs = await getBlockTimestamp(
		block.block.header.hash.toString(),
		getHostStateMachine(chainId),
	)

	await BandwidthService.recordEviction({ chain, appHex, tier, lostBytes, blockTimestampMs })

	await BandwidthService.syncActiveCounts({
		chain,
		appHex,
		palletAppChain: data[0],
		palletAppKey: data[1],
		blockHash: block.block.header.hash.toString(),
		blockTimestampMs,
	})
})
