import { TOKEN_REGISTRY, TokenConfig } from "@/addresses/token-registry.addresses"
import { TokenRegistry } from "@/configs/src/types"
import { normalizeTimestamp, timestampToDate } from "@/utils/date.helpers"
import { fulfilled, safeArray } from "@/utils/data.helper"

/**
 * Token Registry Service manages token configurations and metadata,
 * providing a centralized repository for token information.
 */
export class TokenRegistryService {
	/**
	 * Initialize token registry with default tokens from TOKEN_REGISTRY.
	 * Only processes new tokens that don't exist in the database.
	 * @param currentTimestamp - Current timestamp
	 */
	static async initialize(currentTimestamp: bigint): Promise<void> {
		logger.info(`[TokenRegistryService.initialize] Initializing token registry`)

		try {
			const storedTokens = await this.getTokens()
			const tokenConfigs = safeArray(TOKEN_REGISTRY)

			const storedSymbols = new Set(storedTokens.map((token) => token.symbol))

			const newTokenConfigs = tokenConfigs.filter((config) => !storedSymbols.has(config.symbol))
			if (newTokenConfigs.length === 0) {
				return
			}

			const promises = newTokenConfigs.map((config) => this.getOrCreateToken(config, currentTimestamp))

			const results = await Promise.allSettled(promises)
			const successful = fulfilled(results)

			logger.info(`[TokenRegistryService.initialize] Successfully registered ${successful.length} new tokens`)
		} catch (error) {
			// @ts-ignore
			throw new Error(`Token registry initialization failed: ${error.message}`)
		}
	}

	/**
	 * Register or update a token in the registry
	 * @param tokenConfig - Token configuration
	 * @param currentTimestamp - Current timestamp
	 */
	static async getOrCreateToken(config: TokenConfig, currentTimestamp: bigint): Promise<TokenRegistry> {
		const { name, symbol, updateFrequencySeconds, address } = config

		let token = await this.get(symbol)
		if (!token) {
			token = TokenRegistry.create({
				id: symbol,
				name,
				symbol,
				updateFrequencySeconds,
				address,
				lastUpdatedAt: normalizeTimestamp(currentTimestamp),
				createdAt: timestampToDate(currentTimestamp),
			})
		}

		token.name = name
		token.address = address
		token.updateFrequencySeconds = updateFrequencySeconds
		token.lastUpdatedAt = normalizeTimestamp(currentTimestamp)

		await token.save()

		return token
	}

	/**
	 * Check if the token needs price update based on its update frequency
	 * @param symbol - Token symbol
	 * @param lastPriceUpdate - Token price last updated timestamp in bigint
	 * @param currentTimestamp - Current timestamp in bigint
	 * @returns Boolean indicating if token needs update
	 */
	static async isStale(token: TokenRegistry, lastPriceUpdate: bigint, currentTimestamp: bigint): Promise<boolean> {
		const timeSinceUpdateMs = Number(normalizeTimestamp(currentTimestamp)) - Number(lastPriceUpdate)
		const frequencyMs = token.updateFrequencySeconds * 1000 // Convert to milliseconds
		const needsUpdate = timeSinceUpdateMs >= frequencyMs

		logger.debug(
			`[TokenRegistryService.isStale] Token ${token.symbol}: timeSinceUpdate=${timeSinceUpdateMs}ms, frequency=${frequencyMs}ms, needsUpdate=${needsUpdate}`,
		)

		return needsUpdate
	}

	/**
	 * getTokens fetches all tokens from the database with pagination.
	 * @param limit - Optional page size (defaults to 1000 for better performance)
	 * @returns Promise<TokenRegistry[]> - Array of all tokens
	 */
	static async getTokens(limit: number = 100): Promise<TokenRegistry[]> {
		const allTokens: TokenRegistry[] = []
		let offset = 0
		let totalFetched = 0

		try {
			while (true) {
				const startTime = Date.now()
				const tokens = await TokenRegistry.getByFields([], { limit, offset })
				const fetchTime = Date.now() - startTime

				if (tokens.length === 0) {
					break
				}

				allTokens.push(...tokens)
				totalFetched += tokens.length
				offset += limit

				if (tokens.length < limit) {
					break
				}
			}

			logger.info(`[TokenRegistryService.getTokens] Successfully fetched ${totalFetched} tokens total`)
			return allTokens
		} catch (error) {
			logger.error(`[TokenRegistryService.getTokens] Error fetching tokens at offset ${offset}:`, error)
			throw new Error(`Failed to fetch tokens: ${error instanceof Error ? error.message : "Unknown error"}`)
		}
	}

	/**
	 * get fetches a token by symbol
	 * @param symbol
	 * @returns
	 */
	static async get(symbol: string): ReturnType<typeof TokenRegistry.get> {
		return TokenRegistry.get(symbol)
	}
}
