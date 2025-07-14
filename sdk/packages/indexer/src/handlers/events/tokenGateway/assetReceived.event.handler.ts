import Decimal from "decimal.js"
import stringify from "safe-stable-stringify"

import { AssetReceivedLog } from "@/configs/src/types/abi-interfaces/TokenGatewayAbi"
import { TokenGatewayService } from "@/services/tokenGateway.service"
import { CumulativeVolumeUSD, TeleportStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import PriceHelper from "@/utils/price.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"

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

	const tokenContract = await TokenGatewayService.getAssetTokenContract(assetId.toString())
	const decimals = await tokenContract.decimals()
	const symbol = await tokenContract.symbol()

	const usdValue = await PriceHelper.getTokenPriceInUSDCoingecko(symbol, amount.toBigInt(), decimals)

	// Count the volume in USD
	let cumulativeVolumeUSD = await CumulativeVolumeUSD.get(`TokenGateway`)
	if (!cumulativeVolumeUSD) {
		cumulativeVolumeUSD = CumulativeVolumeUSD.create({
			id: `TokenGateway`,
			volumeUSD: new Decimal(usdValue.amountValueInUSD).toFixed(18),
			lastUpdatedAt: timestamp,
		})
	}

	if (cumulativeVolumeUSD.lastUpdatedAt !== timestamp) {
		cumulativeVolumeUSD.volumeUSD = new Decimal(cumulativeVolumeUSD.volumeUSD)
			.plus(new Decimal(usdValue.amountValueInUSD))
			.toFixed(18)
	}

	await TokenGatewayService.updateTeleportStatus(commitment, TeleportStatus.RECEIVED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
}
