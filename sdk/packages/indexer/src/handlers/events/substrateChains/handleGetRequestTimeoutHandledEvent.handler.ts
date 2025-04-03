import { SubstrateEvent } from "@subql/types"
import { Status } from "@/configs/src/types"
import { GetRequest } from "@/configs/src/types/models"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"

export async function handleSubstrateGetRequestTimeoutHandledEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw Ismp.GetRequestTimeoutHandled Event on ${getHostStateMachine(chainId)}`)

	const host = getHostStateMachine(chainId)

	if (!event.extrinsic) return

	const {
		event: { data },
		extrinsic,
		block: {
			timestamp,
			block: {
				header: { number: blockNumber, hash: blockHash },
			},
		},
	} = event

	const eventData = data.toJSON()
	const timeoutData = Array.isArray(eventData)
		? (eventData[0] as { commitment: any; source: any; dest: any })
		: undefined

	if (!timeoutData) {
		logger.error(`Could not parse event data for ${extrinsic.extrinsic.hash.toString()}`)
		return
	}

	const request = await GetRequest.get(timeoutData.commitment.toString())
	if (!request) {
		logger.error(`Get Request not found for commitment ${timeoutData.commitment.toString()}`)
		return
	}

	let timeoutStatus: Status
	if (request.source === host) {
		timeoutStatus = Status.TIMED_OUT
	} else {
		timeoutStatus = Status.HYPERBRIDGE_TIMED_OUT
	}

	await GetRequestService.updateStatus({
		commitment: timeoutData.commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp: timestamp ? BigInt(Date.parse(timestamp.toString())) : BigInt(0),
		status: timeoutStatus,
		transactionHash: extrinsic.extrinsic.hash.toString(),
	})
}
