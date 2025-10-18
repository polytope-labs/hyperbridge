import Decimal from "decimal.js"

import { ERC6160Ext20Abi__factory, TokenGatewayAbi__factory } from "@/configs/src/types/contracts"
import {
	TeleportStatus,
	TeleportStatusMetadata,
	TokenGatewayAssetTeleported,
	ProtocolParticipantType,
	PointsActivityType,
} from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"
import { PointsService } from "./points.service"
import { TokenPriceService } from "./token-price.service"
import PriceHelper from "@/utils/price.helpers"
import { TOKEN_GATEWAY_ADDRESSES } from "@/token-gateway-addresses"
import { getOrCreateUser } from "./userActivity.services"

export interface IAssetDetails {
	erc20_address: string
	erc6160_address: string
	is_erc20: boolean
	is_erc6160: boolean
}

export interface ITeleportParams {
	to: string
	dest: string
	amount: bigint
	commitment: string
	from: string
	assetId: string
	redeem: boolean
}

export class TokenGatewayService {
	/**
	 * Get asset details
	 */
	static async getAssetDetails(asset_id: string): Promise<IAssetDetails> {
		const TOKEN_GATEWAY_CONTRACT_ADDRESS = TOKEN_GATEWAY_ADDRESSES[`EVM-${chainId}`]
		const tokenGatewayContract = TokenGatewayAbi__factory.connect(TOKEN_GATEWAY_CONTRACT_ADDRESS, api)

		const erc20Address = await tokenGatewayContract.erc20(asset_id)
		const erc6160Address = await tokenGatewayContract.erc6160(asset_id)

		return {
			erc20_address: erc20Address,
			erc6160_address: erc6160Address,
			is_erc20: !erc20Address.includes("0x" + "0".repeat(39)),
			is_erc6160: !erc6160Address.includes("0x" + "0".repeat(39)),
		}
	}

	/**
	 * Get or create a teleport record
	 */
	static async getOrCreate(
		teleportParams: ITeleportParams,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<TokenGatewayAssetTeleported> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let teleport = await TokenGatewayAssetTeleported.get(teleportParams.commitment)

		const tokenDetails = await this.getAssetDetails(teleportParams.assetId.toString())
		const tokenAddress = tokenDetails.is_erc20 ? tokenDetails.erc20_address : tokenDetails.erc6160_address

		const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
		const decimals = await tokenContract.decimals()
		const symbol = await tokenContract.symbol()

		const price = await TokenPriceService.getPrice(symbol, timestamp)
		const { amountValueInUSD } = PriceHelper.getAmountValueInUSD(teleportParams.amount, decimals, price)

		if (!teleport) {
			teleport = TokenGatewayAssetTeleported.create({
				id: teleportParams.commitment,
				from: teleportParams.from,
				sourceChain: chainId,
				destChain: teleportParams.dest,
				commitment: teleportParams.commitment,
				amount: teleportParams.amount,
				assetId: teleportParams.assetId.toString(),
				to: teleportParams.to,
				redeem: teleportParams.redeem,
				status: TeleportStatus.TELEPORTED,
				usdValue: amountValueInUSD,
				createdAt: timestampToDate(timestamp),
				blockNumber: BigInt(blockNumber),
				blockTimestamp: timestamp,
				transactionHash,
			})
			await teleport.save()

			// Award points for token teleport - using USD value directly
			const teleportValue = new Decimal(amountValueInUSD)
			const pointsToAward = teleportValue.floor().toNumber()

			await PointsService.awardPoints(
				teleportParams.from,
				chainId,
				BigInt(pointsToAward),
				ProtocolParticipantType.USER,
				PointsActivityType.TOKEN_TELEPORTED_POINTS,
				transactionHash,
				`Points awarded for teleporting token ${teleportParams.assetId} with value ${amountValueInUSD} USD`,
				timestamp,
			)

			const user = await getOrCreateUser(teleportParams.from)
			user.totalTeleports = user.totalTeleports + BigInt(1)
			user.totalTeleportedVolumeUSD = new Decimal(user.totalTeleportedVolumeUSD)
				.plus(new Decimal(amountValueInUSD))
				.toString()
			// Optimistically update the total successful teleports and volume
			user.totalSuccessfulTeleports = user.totalSuccessfulTeleports + BigInt(1)
			user.totalSuccessfulTeleportedVolumeUSD = new Decimal(user.totalSuccessfulTeleportedVolumeUSD)
				.plus(new Decimal(amountValueInUSD))
				.toString()
			await user.save()
		}

		return teleport
	}

	/**
	 * Get teleport by commitment
	 */
	static async getByCommitment(commitment: string): Promise<TokenGatewayAssetTeleported | undefined> {
		const teleport = await TokenGatewayAssetTeleported.get(commitment)
		return teleport
	}

	/**
	 * Update teleport status
	 */
	static async updateTeleportStatus(
		commitment: string,
		status: TeleportStatus,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp } = logsData

		const teleport = await TokenGatewayAssetTeleported.get(commitment)

		if (teleport) {
			teleport.status = status
			await teleport.save()

			// Deduct points when teleport is refunded
			if (status === TeleportStatus.REFUNDED) {
				const teleportValue = new Decimal(teleport.usdValue)
				const pointsToDeduct = teleportValue.floor().toNumber()

				await PointsService.deductPoints(
					teleport.from,
					teleport.sourceChain,
					BigInt(pointsToDeduct),
					ProtocolParticipantType.USER,
					PointsActivityType.TOKEN_TELEPORTED_POINTS,
					transactionHash,
					`Points deducted for refunded teleport ${commitment} with value ${teleport.usdValue} USD`,
					timestamp,
				)

				const user = await getOrCreateUser(teleport.from)
				user.totalSuccessfulTeleports = user.totalSuccessfulTeleports - BigInt(1)
				user.totalSuccessfulTeleportedVolumeUSD = new Decimal(user.totalSuccessfulTeleportedVolumeUSD)
					.minus(new Decimal(teleport.usdValue))
					.toString()
				await user.save()
			}

			const teleportStatusMetadata = await TeleportStatusMetadata.create({
				id: `${commitment}.${status}`,
				status,
				chain: `EVM-${chainId}`,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				teleportId: teleport?.id ?? "",
				createdAt: timestampToDate(timestamp),
			})

			await teleportStatusMetadata.save()
		}
	}

	static async getAssetTokenContract(assetId: string) {
		const tokenDetails = await TokenGatewayService.getAssetDetails(assetId.toString())
		const tokenAddress = tokenDetails.is_erc20 ? tokenDetails.erc20_address : tokenDetails.erc6160_address

		return ERC6160Ext20Abi__factory.connect(tokenAddress, api)
	}
}
