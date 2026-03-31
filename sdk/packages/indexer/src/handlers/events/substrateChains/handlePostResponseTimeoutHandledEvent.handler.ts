import { SubstrateEvent } from "@subql/types"

import { ResponseService } from "@/services/response.service"
import { Status } from "@/configs/src/types"
import { getHostStateMachine, isHyperbridge } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { wrap } from "@/utils/event.utils"

export const handleSubstratePostResponseTimeoutHandledEvent = wrap(async (event: SubstrateEvent): Promise<void> => {
	logger.info(`Saw Ismp.PostResponseTimeoutHandled Event on ${getHostStateMachine(chainId)}`)

	if (!event.extrinsic && event.event.data) return

	const {
		event: { data },
		extrinsic,
		block: {
			block: {
				header: { number: blockNumber, hash: blockHash },
			},
		},
	} = event

	const host = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash.toString(), host)

	const timeoutStatus = isHyperbridge(host) ? Status.HYPERBRIDGE_TIMED_OUT : Status.TIMED_OUT

	const eventData = data.toJSON()
	const timeoutData = Array.isArray(eventData)
		? (eventData[0] as { commitment: any; source: any; dest: any })
		: undefined

	if (!timeoutData) return

	await ResponseService.updateStatus({
		commitment: timeoutData.commitment.toString(),
		chain: host,
		blockNumber: blockNumber.toString(),
		blockHash: blockHash.toString(),
		blockTimestamp,
		status: timeoutStatus,
		transactionHash: extrinsic!.extrinsic.hash.toString(),
	})
})
