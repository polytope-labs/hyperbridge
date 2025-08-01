import { SubstrateEvent } from "@subql/types"
import { RequestService } from "@/services/request.service"
import { Status } from "@/configs/src/types"
import { getHostStateMachine, isHyperbridge } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Request } from "@/configs/src/types/models"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { stringify } from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"

type EventData = {
	commitment: string
	relayer: string
}
export const handleSubstratePostRequestHandledEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	logger.info(`Saw Ismp.PostRequestHandled Event on ${getHostStateMachine(chainId)}`)

	if (!event.extrinsic && event.event.data) return

	const {
		extrinsic,
		block: {
			block: {
				header: { number: blockNumber, hash: blockHash },
			},
		},
	} = event

	const eventData = event.event.data[0] as unknown as EventData
	const relayer_id = eventData.relayer.toString()

	logger.info(`Handling ISMP PostRequestHandled Event Data: ${stringify({ eventData })}`)

	const host = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash.toString(), host)

	const request = await Request.get(eventData.commitment.toString())

	if (!request) {
		logger.error(`Request not found for commitment ${eventData.commitment.toString()}`)
		return
	}

	let status: Status
	if (request.dest === host) {
		status = Status.DESTINATION
	} else {
		status = Status.HYPERBRIDGE_DELIVERED
	}

	logger.info(`Updating Hyperbridge chain stats for ${host}`)
	await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, host, blockTimestamp)

	logger.info(
		`Handling ISMP PostRequestHandled Event: ${stringify({
			commitment: eventData.commitment.toString(),
			chain: host,
			blockNumber: blockNumber,
			blockHash: blockHash,
			blockTimestamp,
			status,
			transactionHash: extrinsic?.extrinsic.hash || "",
		})}`,
	)

	await RequestService.updateStatus({
		commitment: eventData.commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp,
		status,
		transactionHash: extrinsic?.extrinsic.hash.toString() || "",
	})
})
