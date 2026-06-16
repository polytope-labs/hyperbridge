import Decimal from "decimal.js"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { ProtocolDustCollected } from "@/configs/src/types/models/ProtocolDustCollected"
import { ProtocolDustSwept } from "@/configs/src/types/models/ProtocolDustSwept"
import { timestampToDate } from "@/utils/date.helpers"
import PriceHelper from "@/utils/price.helpers"
import { TokenPriceService } from "./token-price.service"
import stringify from "safe-stable-stringify"

export class ProtocolRevenueService {
	/**
	 * Get or create a DustCollected record
	 */
	static async recordDustCollected(
		chain: string,
		tokenAddress: string,
		amount: bigint,
		timestamp: bigint,
	): Promise<ProtocolDustCollected> {
		const id = `${chain}-${tokenAddress.toLowerCase()}`
		let symbol = "eth"

		// Get token symbol if not native token
		if (tokenAddress.toLowerCase() !== "0x0000000000000000000000000000000000000000") {
			try {
				const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
				symbol = await tokenContract.symbol()
			} catch (error) {
				logger.warn(
					`Failed to get symbol for token ${tokenAddress}: ${stringify({
						error: error as unknown as Error,
					})}`,
				)
				symbol = "UNKNOWN"
			}
		}

		let dustCollected = await ProtocolDustCollected.get(id)

		if (!dustCollected) {
			dustCollected = await ProtocolDustCollected.create({
				id,
				chain,
				tokenSymbol: symbol,
				amount,
				lastUpdated: timestampToDate(timestamp),
			})
		} else {
			dustCollected.amount = dustCollected.amount + amount
			dustCollected.lastUpdated = timestampToDate(timestamp)
		}

		await dustCollected.save()

		logger.info(
			`DustCollected recorded: ${stringify({
				id,
				tokenSymbol: symbol,
				amount: dustCollected.amount.toString(),
			})}`,
		)

		return dustCollected
	}

	/**
	 * Get or create a DustSwept record
	 */
	static async recordDustSwept(
		chain: string,
		tokenAddress: string,
		amount: bigint,
		timestamp: bigint,
	): Promise<ProtocolDustSwept> {
		const id = `${chain}-${tokenAddress.toLowerCase()}`
		let symbol = "eth"

		// Get token symbol if not native token
		if (tokenAddress.toLowerCase() !== "0x0000000000000000000000000000000000000000") {
			try {
				const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
				symbol = await tokenContract.symbol()
			} catch (error) {
				logger.warn(
					`Failed to get symbol for token ${tokenAddress}: ${stringify({
						error: error as unknown as Error,
					})}`,
				)
				symbol = "UNKNOWN"
			}
		}

		let dustSwept = await ProtocolDustSwept.get(id)

		if (!dustSwept) {
			dustSwept = await ProtocolDustSwept.create({
				id,
				chain,
				tokenSymbol: symbol,
				amount,
				lastUpdated: timestampToDate(timestamp),
			})
		} else {
			dustSwept.amount = dustSwept.amount + amount
			dustSwept.lastUpdated = timestampToDate(timestamp)
		}

		await dustSwept.save()

		logger.info(
			`DustSwept recorded: ${stringify({
				id,
				tokenSymbol: symbol,
				amount: dustSwept.amount.toString(),
			})}`,
		)

		return dustSwept
	}
}
