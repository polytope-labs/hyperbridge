import stringify from "safe-stable-stringify"
import { TokenPrice, TokenPriceLog } from "@/configs/src/types"
import { normalizeTimestamp, timestampToDate } from "@/utils/date.helpers"
import PriceHelper from "@/utils/price.helpers"
import { fulfilled } from "@/utils/data.helper"
import { ErrTokenPriceUnavailable } from "@/types/errors"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { TESTNET_STATE_MACHINE_IDS } from "@/testnet-state-machine-ids"

import { TokenRegistryService } from "./token-registry.service"
import { TokenConfig } from "@/addresses/token-registry.addresses"

const DEFAULT_PROVIDER = "COINGECKO" as const

/**
 * Check if current chain is a testnet chain
 */
function isTestnetChain(): boolean {
	try {
		const currentStateMachineId = getHostStateMachine(chainId)
		return TESTNET_STATE_MACHINE_IDS.includes(currentStateMachineId)
	} catch (error) {
		// If we can't determine the state machine ID, assume it's not testnet
		return false
	}
}

/**
 * Token Price Service fetches prices from CoinGecko adapter and stores them in the TokenPrice (current) and TokenPriceLog (historical).
 */
export class TokenPriceService {
	/**
	 * getPrice fetches the current price for a token
	 * @param symbol - The symbol of the token to fetch the price for
	 * @returns A Promise that resolves to the price as a number
	 */
	static async getPrice(symbol: string, currentTimestamp = BigInt(Date.now())): Promise<number> {
		// Return zero price for testnet chains
		if (isTestnetChain()) {
			logger.info(`[TokenPriceService.getPrice] Returning zero price for testnet chain: ${symbol}`)
			return 0
		}

		try {
			let token = await TokenRegistryService.get(symbol)
			if (!token) {
				const tokenConfig = { name: symbol, symbol, updateFrequencySeconds: 600 } as TokenConfig
				token = await TokenRegistryService.getOrCreateToken(tokenConfig, currentTimestamp)
			}

			let tokenPrice = await TokenPrice.get(symbol)
			if (!tokenPrice) {
				const updatedTokenPrices = await this.updateTokenPrices([symbol], currentTimestamp)
				if (updatedTokenPrices instanceof Error) {
					logger.error(
						`[TokenPriceService.getPrice] Failed to update token price for ${symbol}`,
						updatedTokenPrices,
					)
					return 0
				}
				if (updatedTokenPrices.length === 0) {
					logger.error(`[TokenPriceService.getPrice] No token prices updated for ${symbol}`)
					return 0
				}
				tokenPrice = updatedTokenPrices[0]
			}

			const stale = await TokenRegistryService.isStale(token, tokenPrice.lastUpdatedAt, currentTimestamp)
			if (!stale) return parseFloat(tokenPrice.price)

			const updatedTokenPrices = await this.updateTokenPrices([symbol], currentTimestamp)
			if (updatedTokenPrices instanceof Error) {
				logger.error(
					`[TokenPriceService.getPrice] Failed to update stale token price for ${symbol}`,
					updatedTokenPrices,
				)
				return 0
			}
			if (updatedTokenPrices.length === 0) {
				logger.error(`[TokenPriceService.getPrice] No token prices updated for stale ${symbol}`)
				return 0
			}

			return parseFloat(tokenPrice.price)
		} catch (error) {
			if (ErrTokenPriceUnavailable.isError(error)) {
				logger.warn(`[TokenPriceService.getPrice] Price unavailable for ${symbol}, returning 0`)
				return 0
			}

			logger.error(`[TokenPriceService.getPrice] Failed to get token price for ${symbol}`, error)
			return 0
		}
	}

	/**
	 * storeTokenPrice creates or updates a TokenPrice entity and creates a TokenPriceLog entry
	 * @param symbol - Token symbol
	 * @param price - Price value
	 * @param blockTimestamp - Block timestamp
	 */
	static async storeTokenPrice(symbol: string, price: number, blockTimestamp: bigint): Promise<TokenPrice> {
		const normalizedTimestamp = normalizeTimestamp(blockTimestamp)

		let tokenPrice = await TokenPrice.get(symbol)
		if (!tokenPrice) {
			tokenPrice = TokenPrice.create({
				id: symbol,
				symbol,
				currency: "USD",
				price: price.toString(),
				lastUpdatedAt: normalizedTimestamp,
			})
		}

		tokenPrice.price = price.toString()
		tokenPrice.lastUpdatedAt = normalizedTimestamp
		logger.debug(`[TokenPriceService.storeTokenPrice] Updating price entry: ${symbol}`)

		const tokenPriceLog = TokenPriceLog.create({
			id: `${symbol}-${blockTimestamp}`,
			symbol,
			currency: "USD",
			price: price.toString(),
			provider: DEFAULT_PROVIDER,
			timestamp: normalizedTimestamp,
			createdAt: timestampToDate(blockTimestamp),
		})

		await tokenPrice.save()
		await tokenPriceLog.save()

		return tokenPrice
	}

	static async initializePriceIndexing(currentTimestamp: bigint): Promise<void> {
		await TokenRegistryService.initialize(currentTimestamp)
		await this.syncAllTokenPrices(currentTimestamp)
	}

	/**
	 * syncAllTokenPrices updates prices for all tokens that require updates
	 * @param currentTimestamp - Current timestamp
	 * @param currency - Currency to update (defaults to USD)
	 */
	static async syncAllTokenPrices(currentTimestamp: bigint): Promise<void> {
		const tokens = await TokenRegistryService.getTokens()

		const tokensToUpdate = tokens.map(async (token) => {
			const tokenPrice = await TokenPrice.get(token.symbol)
			if (!tokenPrice) {
				return token.symbol
			}

			const isStale = await TokenRegistryService.isStale(token, tokenPrice.lastUpdatedAt, currentTimestamp)
			return isStale ? token.symbol : null
		})

		const checkResults = await Promise.allSettled(tokensToUpdate)
		const symbolsNeedingUpdate = fulfilled(checkResults).filter((t) => t !== null)
		if (symbolsNeedingUpdate.length === 0) {
			return
		}

		const result = await this.updateTokenPrices(symbolsNeedingUpdate, currentTimestamp)
		if (result instanceof Error) {
			logger.error(`[TokenPriceService.syncAllTokenPrices] Failed to update token prices`, result)
		}
	}

	/**
	 * updateTokenPrices fetches prices from CoinGecko and stores them
	 * @param symbols - Array of token symbols to update
	 * @param currencies - Currencies to store prices (optional)
	 * @param blockTimestamp - Timestamp of the block to update prices for (optional)
	 */
	static async updateTokenPrices(symbols: string[], blockTimestamp: bigint): Promise<TokenPrice[] | Error> {
		logger.info(`[TokenPriceService.updateTokenPrices] Syncing prices for: ${symbols}`)

		const response = await PriceHelper.getTokenPriceFromCoinGecko(symbols)
		if (response instanceof Error) {
			return response
		}

		logger.info(`[TokenPriceService.updateTokenPrices] CoinGecko response: ${stringify(response)}`)

		const storePromises = symbols.flatMap((symbol) => {
			const prices = (response[symbol.toLowerCase()] || response[symbol.toUpperCase()])?.usd
			if (!prices) return []

			return this.storeTokenPrice(symbol, prices, blockTimestamp)
		})

		const updatedTokensPromise = await Promise.allSettled(storePromises)
		return fulfilled(updatedTokensPromise)
	}
}
