import Decimal from "decimal.js"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { ProtocolDustCollected } from "@/configs/src/types/models/ProtocolDustCollected"
import { ProtocolDustSwept } from "@/configs/src/types/models/ProtocolDustSwept"
import { CumulativeDustCollectedPerChain } from "@/configs/src/types/models/CumulativeDustCollectedPerChain"
import { CumulativeDustSweptPerChain } from "@/configs/src/types/models/CumulativeDustSweptPerChain"
import { timestampToDate } from "@/utils/date.helpers"
import PriceHelper from "@/utils/price.helpers"
import { toScaledUsd } from "./volume.service"
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
		let decimals = 18

		// Get token symbol and decimals if not native token
		if (tokenAddress.toLowerCase() !== "0x0000000000000000000000000000000000000000") {
			try {
				const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
				symbol = await tokenContract.symbol()
				decimals = await tokenContract.decimals()
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

		const usdDelta = await this.computeDustUsdDelta(chain, tokenAddress, amount, decimals)
		if (usdDelta && usdDelta > 0n) {
			let cumulative = await CumulativeDustCollectedPerChain.get(chain)
			if (!cumulative) {
				cumulative = CumulativeDustCollectedPerChain.create({
					id: chain,
					chain,
					amountUSD: usdDelta,
					lastUpdatedAt: timestamp,
				})
			} else {
				cumulative.amountUSD = cumulative.amountUSD + usdDelta
				cumulative.lastUpdatedAt = timestamp
			}
			await cumulative.save()
		}

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
		let decimals = 18

		// Get token symbol and decimals if not native token
		if (tokenAddress.toLowerCase() !== "0x0000000000000000000000000000000000000000") {
			try {
				const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
				symbol = await tokenContract.symbol()
				decimals = await tokenContract.decimals()
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

		const usdDelta = await this.computeDustUsdDelta(chain, tokenAddress, amount, decimals)
		if (usdDelta && usdDelta > 0n) {
			let cumulative = await CumulativeDustSweptPerChain.get(chain)
			if (!cumulative) {
				cumulative = CumulativeDustSweptPerChain.create({
					id: chain,
					chain,
					amountUSD: usdDelta,
					lastUpdatedAt: timestamp,
				})
			} else {
				cumulative.amountUSD = cumulative.amountUSD + usdDelta
				cumulative.lastUpdatedAt = timestamp
			}
			await cumulative.save()
		}

		logger.info(
			`DustSwept recorded: ${stringify({
				id,
				tokenSymbol: symbol,
				amount: dustSwept.amount.toString(),
			})}`,
		)

		return dustSwept
	}

	/**
	 * Convert a newly-collected/swept token amount to a scaled-1e18 USD bigint using the
	 * on-chain DEX price. Returns null when no price is available, in which case the caller
	 * skips the USD rollup; the raw amount is still retained on the per-token entity.
	 */
	private static async computeDustUsdDelta(
		chain: string,
		tokenAddress: string,
		amount: bigint,
		decimals: number,
	): Promise<bigint | null> {
		const { amountValueInUSD } = await PriceHelper.getTokenPriceInUSDUniswap(tokenAddress, amount, decimals)
		if (!amountValueInUSD || new Decimal(amountValueInUSD).isZero()) {
			logger.warn(
				`[ProtocolRevenueService] No DEX price for ${tokenAddress} on ${chain}; skipping USD rollup, raw amount retained`,
			)
			return null
		}

		return toScaledUsd(amountValueInUSD)
	}
}
