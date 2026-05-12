import { SubstrateEvent } from "@subql/types"
import { Balance } from "@polkadot/types/interfaces"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { BandwidthService } from "@/services/bandwidth.service"

/**
 * `pallet-bandwidth :: BandwidthConsumed` — fired by the gate per
 * inbound message after it deducts bytes from the FIFO head.
 *
 * Payload order:
 *   0. source:    StateMachine (the credit chain)
 *   1. app:       AppKey
 *   2. bytes:     u128
 *   3. remaining: u128 (post-deduct sum; unused here)
 */
export const handleBandwidthConsumedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
	} = event
	logger.info(`Saw Bandwidth.BandwidthConsumed on ${getHostStateMachine(chainId)}`)

	const chain = formatChain(data[0].toJSON())
	const appHex = BandwidthService.normalizeAppHex(data[1].toHex())
	const bytes = (data[2] as unknown as Balance).toBigInt()

	const blockTimestampMs = await getBlockTimestamp(
		block.block.header.hash.toString(),
		getHostStateMachine(chainId),
	)

	await BandwidthService.recordConsumption({ chain, appHex, bytes, blockTimestampMs })

	await BandwidthService.syncActiveCounts({
		chain,
		appHex,
		palletAppChain: data[0],
		palletAppKey: data[1],
		blockHash: block.block.header.hash.toString(),
		blockTimestampMs,
	})
})
