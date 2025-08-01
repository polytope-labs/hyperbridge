import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { AssetTeleportedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { TeleportStatus } from "@/configs/src/types"
import { getHostStateMachine, isSubstrateChain } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"
import { VolumeService } from "@/services/volume.service"
import PriceHelper from "@/utils/price.helpers"

export const handleAssetTeleportedEvent = wrap(async (event: AssetTeleportedLog): Promise<void> => {
	logger.info(`Asset Teleported Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, block, blockHash } = event
	const { to, dest, amount, commitment, from, assetId, redeem } = args!

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	if (isSubstrateChain(dest)) {
		const tokenContract = await TokenGatewayService.getAssetTokenContract(assetId.toString())
		const decimals = await tokenContract.decimals()
		const symbol = await tokenContract.symbol()

		const usdValue = await PriceHelper.getTokenPriceInUSDCoingecko(symbol, amount.toBigInt(), decimals)

		await VolumeService.updateVolume("TokenGateway", usdValue.amountValueInUSD, timestamp)
		return
	}

	logger.info(
		`Asset Teleported Event: ${stringify({
			to,
			dest,
			amount,
			commitment,
			from,
			assetId,
			redeem,
		})}`,
	)

	await TokenGatewayService.getOrCreate(
		{
			to,
			dest,
			amount: amount.toBigInt(),
			commitment,
			from,
			assetId,
			redeem,
		},
		{
			transactionHash,
			blockNumber,
			timestamp,
		},
	)

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.TELEPORTED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
