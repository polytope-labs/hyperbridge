import { SubstrateEvent } from "@subql/types"
import { Status } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { GetRequestService } from "@/services/getRequest.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"

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
			block: {
				header: { number: blockNumber, hash: blockHash },
			},
		},
	} = event

	const eventData = event.event.data[0] as unknown as EventData
	const relayer_id = eventData.relayer.toString()

	logger.info(
		`Handling ISMP GetRequestHandled Event: ${stringify({
			data: event.event.data,
		})}`,
	)

	const host = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash.toString(), host)

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
		blockTimestamp,
		status,
		transactionHash: extrinsic?.extrinsic.hash.toString() || "",
	})
}
