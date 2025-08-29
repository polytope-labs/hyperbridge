import stringify from "safe-stable-stringify"

import { TeleportStatus } from "@/configs/src/types"
import { AssetReceivedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { VolumeService } from "@/services/volume.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { wrap } from "@/utils/event.utils"
import { TokenPriceService } from "@/services/token-price.service"
import PriceHelper from "@/utils/price.helpers"

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

	// NOTE: ERC20 token transfer already indexed via Token Transfer
	const asset = await TokenGatewayService.getAssetDetails(assetId.toString())
	if (!asset.is_erc20) {
		const tokenContract = await TokenGatewayService.getAssetTokenContract(assetId.toString())
		const decimals = await tokenContract.decimals()
		const symbol = await tokenContract.symbol()

		const price = await TokenPriceService.getPrice(symbol, timestamp)
		const { amountValueInUSD } = PriceHelper.getAmountValueInUSD(amount.toBigInt(), decimals, price)

		await VolumeService.updateVolume("TokenGateway", amountValueInUSD, timestamp)
	}

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.RECEIVED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
