import stringify from "safe-stable-stringify"

import { TeleportStatus } from "@/configs/src/types"
import { AssetReceivedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { VolumeService } from "@/services/volume.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import PriceHelper from "@/utils/price.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { wrap } from "@/utils/event.utils"

export const handleAssetReceivedEvent = wrap(async (event: AssetReceivedLog): Promise<void> => {
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

	const tokenContract = await TokenGatewayService.getAssetTokenContract(assetId.toString())
	const decimals = await tokenContract.decimals()
	const symbol = await tokenContract.symbol()

	const usdValue = await PriceHelper.getTokenPriceInUSDCoingecko(symbol, amount.toBigInt(), decimals)

	await VolumeService.updateVolume("TokenGateway", usdValue.amountValueInUSD, timestamp)

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.RECEIVED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
