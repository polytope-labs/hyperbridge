import { SubstrateEvent } from "@subql/types"
import { Balance } from "@polkadot/types/interfaces"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { BandwidthService } from "@/services/bandwidth.service"

/**
 * `pallet-bandwidth :: BandwidthCredited` — fired when an inbound
 * purchase message creates a new subscription on the credit chain.
 *
 * Payload order:
 *   0. app_chain: StateMachine
 *   1. app:       AppKey (BoundedVec<u8, 32>)
 *   2. paid_from: StateMachine
 *   3. tier:      TierIndex
 *   4. bytes:     u128
 *   5. expires_at: u64
 */
export const handleBandwidthCreditedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
		extrinsic,
	} = event
	logger.info(`Saw Bandwidth.BandwidthCredited on ${getHostStateMachine(chainId)}`)

	const chain = formatChain(data[0].toJSON())
	const appHex = BandwidthService.normalizeAppHex(data[1].toHex())
	const paidFrom = formatChain(data[2].toJSON())
	const tier = (data[3] as unknown as { toNumber: () => number }).toNumber()
	const bytes = (data[4] as unknown as Balance).toBigInt()
	const expiresAtSecs = (data[5] as unknown as { toBigInt: () => bigint }).toBigInt()

	const blockTimestampMs = await getBlockTimestamp(
		block.block.header.hash.toString(),
		getHostStateMachine(chainId),
	)

	await BandwidthService.recordCredit({
		chain,
		appHex,
		tier,
		bytes,
		expiresAtSecs,
		paidFrom,
		forced: false,
		blockNumber: block.block.header.number.toBigInt(),
		blockTimestampMs,
		eventIdx: event.idx,
		extrinsicHash: extrinsic?.extrinsic.hash.toString(),
	})

	await BandwidthService.syncActiveCounts({
		chain,
		appHex,
		palletAppChain: data[0],
		palletAppKey: data[1],
		blockHash: block.block.header.hash.toString(),
		blockTimestampMs,
	})
})
