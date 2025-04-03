import { SubstrateEvent } from "@subql/types"
import { Status } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { GetRequest } from "@/configs/src/types/models"
import { GetRequestService } from "@/services/getRequest.service"

type EventData = {
	commitment: string
	relayer: string
}
export async function handleSubstrateGetRequestHandledEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw Ismp.GetRequestHandled Event on ${getHostStateMachine(chainId)}`)

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
		`Handling ISMP GetRequestHandled Event: ${JSON.stringify({
			data: event.event.data,
		})}`,
	)

	const host = getHostStateMachine(chainId)

	const request = await GetRequestService.createOrUpdate({
		id: eventData.commitment.toString(),
	})

	let status: Status
	if (request.source === host) {
		status = Status.DESTINATION
	} else {
		status = Status.HYPERBRIDGE_DELIVERED
	}

	logger.info(`Updating Hyperbridge chain stats for ${host}`)
	await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, host)

	await GetRequestService.updateStatus({
		commitment: eventData.commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp: timestamp ? BigInt(Date.parse(timestamp.toString())) : BigInt(0),
		status,
		transactionHash: extrinsic?.extrinsic.hash.toString() || "",
	})
}
