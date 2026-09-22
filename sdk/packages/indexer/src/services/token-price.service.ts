import { TokenPrice, TokenPriceLog } from "@/configs/src/types"
import { fetchTokenUsdPrice } from "@/services/orderbookRates.service"
import { fulfilled } from "@/utils/data.helper"
import { normalizeTimestamp, timestampToDate } from "@/utils/date.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { TESTNET_STATE_MACHINE_IDS } from "@/testnet-state-machine-ids"

/** What `TokenPriceLog.provider` records: prices come from the HyperFX orderbook. */
const PROVIDER = "HYPERFX" as const

/**
 * How old a stored price may be before it is fetched again.
 *
 * It is not the only thing keeping requests down: the rates service caches each symbol's rate for a
 * minute of its own, so this bounds how often a price is *written*, not how often one is asked for.
 */
export const PRICE_REFRESH_INTERVAL_MS = 600_000

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
 * Check if token price is stale and needs updating
 */
function isPriceStale(symbol: string, lastPriceUpdate: bigint, currentTimestamp: bigint): boolean {
	const timeSinceUpdateMs = Number(normalizeTimestamp(currentTimestamp)) - Number(lastPriceUpdate)
	const needsUpdate = timeSinceUpdateMs >= PRICE_REFRESH_INTERVAL_MS

	logger.debug(
		`[TokenPriceService.isPriceStale] Token ${symbol}: timeSinceUpdate=${timeSinceUpdateMs}ms, frequency=${PRICE_REFRESH_INTERVAL_MS}ms, needsUpdate=${needsUpdate}`,
	)

	return needsUpdate
}

/**
 * Token Price Service prices tokens from the HyperFX orderbook and stores them in the TokenPrice
 * (current) and TokenPriceLog (historical).
 *
 * There is no whitelist: the orderbook decides what it can price, and a token it quotes no rate for
 * is worth 0 here. That is every token without a book against a $1 stable.
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
			// Try to get existing token price
			let tokenPrice = await TokenPrice.get(symbol)
			if (!tokenPrice) {
				// No price exists, fetch and store new price
				const updatedTokenPrices = await this.updateTokenPrices([symbol], currentTimestamp)
				if (updatedTokenPrices.length === 0) {
					logger.error(`[TokenPriceService.getPrice] No token prices updated for ${symbol}`)
					return 0
				}
				tokenPrice = updatedTokenPrices[0]
			}

			// Check if price is stale
			const stale = isPriceStale(symbol, tokenPrice.lastUpdatedAt, currentTimestamp)
			if (!stale) {
				return parseFloat(tokenPrice.price)
			}

			// Price is stale, update it
			const updatedTokenPrices = await this.updateTokenPrices([symbol], currentTimestamp)
			if (updatedTokenPrices.length === 0) {
				logger.error(`[TokenPriceService.getPrice] No token prices updated for stale ${symbol}`)
				// Return the stale price rather than 0
				return parseFloat(tokenPrice.price)
			}

			return parseFloat(updatedTokenPrices[0].price)
		} catch (error) {
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
			provider: PROVIDER,
			timestamp: normalizedTimestamp,
			createdAt: timestampToDate(blockTimestamp),
		})

		await tokenPrice.save()
		await tokenPriceLog.save()

		return tokenPrice
	}

	/**
	 * updateTokenPrices prices symbols from the orderbook and stores them. A symbol the orderbook
	 * quotes no rate for is left out of the result rather than stored at zero, so the caller keeps
	 * whatever price it already held.
	 * @param symbols - Array of token symbols to update
	 * @param blockTimestamp - Timestamp of the block to update prices for
	 * @returns Array of updated TokenPrice entities
	 */
	static async updateTokenPrices(symbols: string[], blockTimestamp: bigint): Promise<TokenPrice[]> {
		logger.info(`[TokenPriceService.updateTokenPrices] Syncing prices for: ${symbols.join(", ")}`)

		const priced = await Promise.all(
			symbols.map(async (symbol) => ({ symbol, price: await fetchTokenUsdPrice(symbol) })),
		)

		const storePromises = priced.flatMap(({ symbol, price }) => {
			if (!price || price.lte(0)) {
				logger.warn(`[TokenPriceService.updateTokenPrices] The orderbook quotes no price for ${symbol}`)
				return []
			}

			return this.storeTokenPrice(symbol, price.toNumber(), blockTimestamp)
		})

		const updatedTokensPromise = await Promise.allSettled(storePromises)
		return fulfilled(updatedTokensPromise)
	}
}
