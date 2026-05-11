import { SubstrateEvent } from "@subql/types"
import { Balance } from "@polkadot/types/interfaces"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { BandwidthService } from "@/services/bandwidth.service"

/**
 * `pallet-bandwidth :: ForceCredited` — fired when admin pushes an
 * out-of-band subscription via `force_credit`. Shape mirrors
 * `BandwidthCredited` minus the cross-chain payer (admin source).
 *
 * Payload order:
 *   0. app_chain:  StateMachine
 *   1. app:        AppKey
 *   2. tier:       TierIndex
 *   3. bytes:      u128
 *   4. expires_at: u64
 */
export const handleForceCreditedEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
		extrinsic,
	} = event
	logger.info(`Saw Bandwidth.ForceCredited on ${getHostStateMachine(chainId)}`)

	const chain = formatChain(data[0].toJSON())
	const appHex = BandwidthService.normalizeAppHex(data[1].toHex())
	const tier = (data[2] as unknown as { toNumber: () => number }).toNumber()
	const bytes = (data[3] as unknown as Balance).toBigInt()
	const expiresAtSecs = (data[4] as unknown as { toBigInt: () => bigint }).toBigInt()

	const blockTimestampMs = await getBlockTimestamp(
		block.block.header.hash.toString(),
		getHostStateMachine(chainId),
	)

	// paidFrom = same as app_chain for admin pushes (no external payer).
	await BandwidthService.recordCredit({
		chain,
		appHex,
		tier,
		bytes,
		expiresAtSecs,
		paidFrom: chain,
		forced: true,
		blockNumber: block.block.header.number.toBigInt(),
		blockTimestampMs,
		eventIdx: event.idx,
		extrinsicHash: extrinsic?.extrinsic.hash.toString(),
	})
})
