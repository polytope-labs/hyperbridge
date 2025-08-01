import { SubstrateEvent } from "@subql/types"
import { ResponseService } from "@/services/response.service"
import { Status } from "@/configs/src/types"
import { getHostStateMachine, isHyperbridge } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"

type EventData = {
	commitment: string
	relayer: string
}

export const handleSubstratePostResponseHandledEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	logger.info(`Saw Ismp.PostResponseHandled Event on ${getHostStateMachine(chainId)}`)

	if (!event.extrinsic && event.event.data) return

	const {
		event: {
			data: [dest_chain, source_chain, request_nonce, commitment, response_commitment],
		},
		extrinsic,
		block: {
			timestamp,
			block: {
				header: { number: blockNumber, hash: blockHash },
			},
		},
	} = event

	const eventData = event.event.data[0] as unknown as EventData
	const relayer_id = eventData.relayer.toString()

	logger.info(
		`Handling ISMP PostRequestHandled Event: ${stringify({
			data: event.event.data,
		})}`,
	)

	const host = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash.toString(), host)

	let status: Status

	// todo: actually check if host is hyperbridge and response source is hyperbridge
	if (isHyperbridge(host)) {
		status = Status.HYPERBRIDGE_DELIVERED
	} else {
		status = Status.DESTINATION
	}

	logger.info(`Updating Hyperbridge chain stats for ${host}`)
	await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, host, blockTimestamp)

	logger.info(
		`Handling ISMP PostRequestHandled Event: ${stringify({
			commitment: response_commitment.toString(),
			chain: host,
			blockNumber: blockNumber,
			blockHash: blockHash,
			blockTimestamp,
			status,
			transactionHash: extrinsic?.extrinsic.hash || "",
		})}`,
	)
	await ResponseService.updateStatus({
		commitment: response_commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp,
		status,
		transactionHash: extrinsic?.extrinsic.hash.toString() || "",
	})
})
