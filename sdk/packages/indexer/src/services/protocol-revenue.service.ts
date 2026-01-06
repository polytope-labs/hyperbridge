import Decimal from "decimal.js"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { DustCollected } from "@/configs/src/types/models/DustCollected"
import { DustSwept } from "@/configs/src/types/models/DustSwept"
import { timestampToDate } from "@/utils/date.helpers"
import PriceHelper from "@/utils/price.helpers"
import { TokenPriceService } from "./token-price.service"
import stringify from "safe-stable-stringify"

export class ProtocolRevenueService {
	/**
	 * Get or create a DustCollected record
	 */
	static async recordDustCollected(tokenAddress: string, amount: bigint, timestamp: bigint): Promise<DustCollected> {
		const id = `${chainId}-${tokenAddress.toLowerCase()}`
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

		let dustCollected = await DustCollected.get(id)

		if (!dustCollected) {
			dustCollected = await DustCollected.create({
				id,
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
	static async recordDustSwept(tokenAddress: string, amount: bigint, timestamp: bigint): Promise<DustSwept> {
		const id = `${chainId}-${tokenAddress.toLowerCase()}`
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

		let dustSwept = await DustSwept.get(id)

		if (!dustSwept) {
			dustSwept = await DustSwept.create({
				id,
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
