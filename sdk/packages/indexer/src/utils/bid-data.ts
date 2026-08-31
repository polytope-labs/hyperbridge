import { SubstrateExtrinsic } from "@subql/types"
import { fetchBidsForOrder, setAggregationFetch } from "@hyperbridge/sdk/intents-helpers"

import { safeFetch } from "@/utils/safeFetch"
import { extractUserOpFromExtrinsic } from "@/utils/extrinsic.helpers"

// The SDK's RPC helpers run inside the SubQuery VM2 sandbox, which has no global `fetch`.
setAggregationFetch(safeFetch)

/**
 * Falls back to the node's offchain storage for a bid's payload.
 *
 * intents_getBidsForOrder pins the chain head internally and has no `at` parameter, and the
 * underlying offchain entry is node-local and expires, so this only returns anything while the bid
 * is still live on the node being queried. It is a best-effort backstop for the extrinsic path, not
 * a replacement for it.
 */
async function fetchBidDataFromRpc(nodeUrl: string, commitment: string, fillerHex: string): Promise<string | undefined> {
	try {
		const bids = await fetchBidsForOrder(nodeUrl, commitment)
		const match = bids.find((bid) => bid.filler?.toLowerCase() === fillerHex.toLowerCase())
		return match?.user_op || undefined
	} catch (err) {
		logger.warn({ err, commitment }, "intents_getBidsForOrder failed for bid enrichment")
		return undefined
	}
}

/**
 * Resolves the raw bid payload behind a BidPlaced event, or undefined if neither source has it.
 *
 * The extrinsic is tried first because it is part of the block being indexed: it is exact (the very
 * bid that raised this event), always present on replay, and costs no network call. The RPC is only
 * a backstop for the case where the call cannot be decoded.
 */
export async function resolveBidData(params: {
	extrinsic?: SubstrateExtrinsic
	commitment: string
	fillerHex: string
	nodeUrl?: string
}): Promise<string | undefined> {
	const { extrinsic, commitment, fillerHex, nodeUrl } = params

	const fromExtrinsic = extractUserOpFromExtrinsic(extrinsic, commitment)
	if (fromExtrinsic) return fromExtrinsic

	if (!nodeUrl) return undefined
	return fetchBidDataFromRpc(nodeUrl, commitment, fillerHex)
}
