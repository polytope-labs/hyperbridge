import { SubstrateEvent } from "@subql/types"

import { wrap } from "@/utils/event.utils"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { resolveBidData } from "@/utils/bid-data"
import { bidPlacedCarriesBidId } from "@/utils/bid-event.helpers"
import { ENV_CONFIG } from "@/constants"
import { FillerBid } from "@/configs/src/types"

/**
 * `pallet-intents-coprocessor :: BidPlaced` — a filler placed a bid on an order commitment.
 *
 * Payload order:
 *   0. filler:     AccountId
 *   1. commitment: H256
 *   2. bid:        H256
 *   3. deposit:    Balance
 *
 * The event carries no bid payload, so it is resolved separately by resolveBidData and stored raw.
 *
 * Events from before the upgrade that added `bid` carry three fields, with the deposit third. They
 * are skipped: see `bidPlacedCarriesBidId`.
 */
export const handleBidPlaced = wrap(async (event: SubstrateEvent): Promise<void> => {
	const {
		event: { data },
		block,
		extrinsic,
	} = event

	if (!bidPlacedCarriesBidId(data)) {
		logger.debug({ blockNumber: block.block.header.number.toString() }, "Skipping pre-upgrade BidPlaced event")
		return
	}

	const [fillerData, commitmentData, bidIdData] = data

	const filler = fillerData.toString()
	const commitment = commitmentData.toHex()
	const bid = bidIdData.toHex()
	const blockNumber = block.block.header.number.toBigInt()

	// A filler may re-bid on the same commitment, so the block number and event index are part of the
	// key — keyed on (commitment, filler) alone, a repeat bid would overwrite its predecessor.
	const id = `${commitment}-${filler}-${blockNumber}-${event.idx}`
	if (await FillerBid.get(id)) return

	const host = getHostStateMachine(chainId)

	const bidData = await resolveBidData({
		extrinsic,
		commitment,
		bid,
		// The RPC keys bids by the raw AccountId bytes, not the SS58 form stored on the entity.
		fillerHex: fillerData.toHex(),
		nodeUrl: replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "") || undefined,
	})

	await FillerBid.create({
		id,
		commitment,
		filler,
		bid,
		bidData,
		extrinsicHash: extrinsic?.extrinsic.hash.toString(),
		blockNumber,
	}).save()

	logger.info({ commitment, filler, bid, blockNumber }, `FillerBid indexed${bidData ? "" : " (no bid data)"}`)
})
