import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { AssetReceivedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { TeleportStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export async function handleAssetReceivedEvent(event: AssetReceivedLog): Promise<void> {
	logger.info(`Asset Received Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash } = event
	const { amount, commitment, from, beneficiary, assetId } = args!

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`Asset Received Event: ${stringify({
			amount,
			commitment,
			from,
			beneficiary,
			assetId,
		})}`,
	)

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.RECEIVED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
}
