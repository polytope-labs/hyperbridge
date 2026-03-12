import stringify from "safe-stable-stringify"
import { TokenPrice, TokenPriceLog } from "@/configs/src/types"
import { normalizeTimestamp, timestampToDate } from "@/utils/date.helpers"
import PriceHelper from "@/utils/price.helpers"
import { fulfilled } from "@/utils/data.helper"
import { ErrTokenPriceUnavailable } from "@/types/errors"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { TESTNET_STATE_MACHINE_IDS } from "@/testnet-state-machine-ids"
import { TOKEN_REGISTRY, TokenConfig } from "@/addresses/token-registry.addresses"

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
 * Get token configuration from TOKEN_REGISTRY
 */
function getTokenConfig(symbol: string): TokenConfig | undefined {
	return TOKEN_REGISTRY.find((cfg) => cfg.symbol === symbol)
}

/**
 * Check if token price is stale and needs updating
 */
function isPriceStale(config: TokenConfig, lastPriceUpdate: bigint, currentTimestamp: bigint): boolean {
	const timeSinceUpdateMs = Number(normalizeTimestamp(currentTimestamp)) - Number(lastPriceUpdate)
	const frequencyMs = config.updateFrequencySeconds * 1000 // Convert to milliseconds
	const needsUpdate = timeSinceUpdateMs >= frequencyMs

	logger.debug(
		`[TokenPriceService.isPriceStale] Token ${config.symbol}: timeSinceUpdate=${timeSinceUpdateMs}ms, frequency=${frequencyMs}ms, needsUpdate=${needsUpdate}`,
	)

	return needsUpdate
}

/**
 * Token Price Service fetches prices from CoinGecko adapter and stores them in the TokenPrice (current) and TokenPriceLog (historical).
 */
export class TokenPriceService {
	/**
	 * getPrice fetches the current price for a token
	 * @param symbol - The symbol of the token to fetch the price for
	 * @param currentTimestamp - Current timestamp in milliseconds
	 * @returns A Promise that resolves to the price as a number
	 */
	static async getPrice(symbol: string, currentTimestamp = BigInt(Date.now())): Promise<number> {
		// Return zero price for testnet chains
		if (isTestnetChain()) {
			logger.info(`[TokenPriceService.getPrice] Returning zero price for testnet chain: ${symbol}`)
			return 0
		}

		try {
			// Check if token is in the whitelist
			const config = getTokenConfig(symbol)
			if (!config) {
				logger.warn(`[TokenPriceService.getPrice] Skipping price for non-whitelisted token: ${symbol}`)
				return 0
			}

			// Try to get existing token price
			let tokenPrice = await TokenPrice.get(symbol)
			if (!tokenPrice) {
				// No price exists, fetch and store new price
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

			// Check if price is stale
			const stale = isPriceStale(config, tokenPrice.lastUpdatedAt, currentTimestamp)
			if (!stale) {
				return parseFloat(tokenPrice.price)
			}

			// Price is stale, update it
			const updatedTokenPrices = await this.updateTokenPrices([symbol], currentTimestamp)
			if (updatedTokenPrices instanceof Error) {
				logger.error(
					`[TokenPriceService.getPrice] Failed to update stale token price for ${symbol}`,
					updatedTokenPrices,
				)
				// Return the stale price rather than 0
				return parseFloat(tokenPrice.price)
			}
			if (updatedTokenPrices.length === 0) {
				logger.error(`[TokenPriceService.getPrice] No token prices updated for stale ${symbol}`)
				// Return the stale price rather than 0
				return parseFloat(tokenPrice.price)
			}

			return parseFloat(updatedTokenPrices[0].price)
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

	/**
	 * initializePriceIndexing syncs all token prices from TOKEN_REGISTRY
	 * @param currentTimestamp - Current timestamp
	 */
	static async initializePriceIndexing(currentTimestamp: bigint): Promise<void> {
		logger.info(`[TokenPriceService.initializePriceIndexing] Initializing price indexing for ${TOKEN_REGISTRY.length} tokens`)
		await this.syncAllTokenPrices(currentTimestamp)
	}

	/**
	 * syncAllTokenPrices updates prices for all tokens that require updates
	 * @param currentTimestamp - Current timestamp
	 */
	static async syncAllTokenPrices(currentTimestamp: bigint): Promise<void> {
		// Check which tokens need price updates
		const tokensToUpdate = await Promise.all(
			TOKEN_REGISTRY.map(async (config) => {
				const tokenPrice = await TokenPrice.get(config.symbol)
				
				// If no price exists, needs update
				if (!tokenPrice) {
					return config.symbol
				}

				// Check if price is stale
				const isStale = isPriceStale(config, tokenPrice.lastUpdatedAt, currentTimestamp)
				return isStale ? config.symbol : null
			})
		)

		const symbolsNeedingUpdate = tokensToUpdate.filter((symbol) => symbol !== null) as string[]
		
		if (symbolsNeedingUpdate.length === 0) {
			logger.info(`[TokenPriceService.syncAllTokenPrices] All token prices are up to date`)
			return
		}

		logger.info(`[TokenPriceService.syncAllTokenPrices] Updating ${symbolsNeedingUpdate.length} token prices`)
		const result = await this.updateTokenPrices(symbolsNeedingUpdate, currentTimestamp)
		if (result instanceof Error) {
			logger.error(`[TokenPriceService.syncAllTokenPrices] Failed to update token prices`, result)
		}
	}

	/**
	 * updateTokenPrices fetches prices from CoinGecko and stores them
	 * @param symbols - Array of token symbols to update
	 * @param blockTimestamp - Timestamp of the block to update prices for
	 * @returns Array of updated TokenPrice entities or Error
	 */
	static async updateTokenPrices(symbols: string[], blockTimestamp: bigint): Promise<TokenPrice[] | Error> {
		logger.info(`[TokenPriceService.updateTokenPrices] Syncing prices for: ${symbols.join(", ")}`)

		const response = await PriceHelper.getTokenPriceFromCoinGecko(symbols)
		if (response instanceof Error) {
			return response
		}

		logger.info(`[TokenPriceService.updateTokenPrices] CoinGecko response: ${stringify(response)}`)

		const storePromises = symbols.flatMap((symbol) => {
			const prices = (response[symbol.toLowerCase()] || response[symbol.toUpperCase()])?.usd
			if (!prices) {
				logger.warn(`[TokenPriceService.updateTokenPrices] No price data for ${symbol}`)
				return []
			}

			return this.storeTokenPrice(symbol, prices, blockTimestamp)
		})

		const updatedTokensPromise = await Promise.allSettled(storePromises)
		return fulfilled(updatedTokensPromise)
	}
}