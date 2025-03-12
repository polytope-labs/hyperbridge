import { SubstrateEvent } from "@subql/types"
import { RequestService } from "@/services/request.service"
import { Status } from "@/configs/src/types"
import { getHostStateMachine, isHyperbridge } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Request } from "@/configs/src/types/models"

type EventData = {
	commitment: string
	relayer: string
}
export async function handleSubstratePostRequestHandledEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw Ismp.PostRequestHandled Event on ${getHostStateMachine(chainId)}`)

	if (!event.extrinsic && event.event.data) return

	const {
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
		`Handling ISMP PostRequestHandled Event: ${JSON.stringify({
			data: event.event.data,
		})}`,
	)

	const host = getHostStateMachine(chainId)

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
	await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, host)

	logger.info(
		`Handling ISMP PostRequestHandled Event: ${JSON.stringify({
			commitment: eventData.commitment.toString(),
			chain: host,
			blockNumber: blockNumber,
			blockHash: blockHash,
			blockTimestamp: timestamp,
			status,
			transactionHash: extrinsic?.extrinsic.hash || "",
		})}`,
	)

	await RequestService.updateStatus({
		commitment: eventData.commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp: timestamp ? BigInt(Date.parse(timestamp.toString())) : BigInt(0),
		status,
		transactionHash: extrinsic?.extrinsic.hash.toString() || "",
	})
}
