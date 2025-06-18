import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { AssetRefundedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { TeleportStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export async function handleAssetRefundedEvent(event: AssetRefundedLog): Promise<void> {
	logger.info(`Asset Refunded Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash } = event
	const { amount, commitment, beneficiary, assetId } = args!

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`Asset Refunded Event: ${stringify({
			amount,
			commitment,
			beneficiary,
			assetId,
		})}`,
	)

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.REFUNDED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
}
