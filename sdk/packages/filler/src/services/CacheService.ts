import { HexString } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"

interface GasEstimateCache {
	fillGas: string
	postGas: string
	relayerFeeInFeeToken: string
	relayerFeeInNativeToken: string
	timestamp: number
}

interface SwapCall {
	to: string
	data: string
	value: string
}

interface SwapOperationsCache {
	calls: SwapCall[]
	totalGasEstimate: string
	timestamp: number
}

interface CacheData {
	gasEstimates: Record<string, GasEstimateCache>
	swapOperations: Record<string, SwapOperationsCache>
	feeTokens: Record<string, { address: HexString; decimals: number }>
	perByteFees: Record<string, Record<string, bigint>>
	tokenDecimals: Record<string, Record<HexString, number>>
}

export class CacheService {
	private cacheData: CacheData
	private readonly CACHE_EXPIRY_MS = 1 * 60 * 1000 // 1 minute
	private logger = getLogger("cache-service")

	constructor() {
		this.cacheData = { gasEstimates: {}, swapOperations: {}, feeTokens: {}, perByteFees: {}, tokenDecimals: {} }
	}

	private isCacheValid(timestamp: number): boolean {
		return Date.now() - timestamp < this.CACHE_EXPIRY_MS
	}

	private cleanupStaleData(): void {
		// Clean up gas estimates
		const staleGasEstimateIds = Object.entries(this.cacheData.gasEstimates)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleGasEstimateIds.forEach((orderId) => {
			delete this.cacheData.gasEstimates[orderId]
		})

		// Clean up swap operations
		const staleSwapOperationIds = Object.entries(this.cacheData.swapOperations)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleSwapOperationIds.forEach((orderId) => {
			delete this.cacheData.swapOperations[orderId]
		})
	}

	getGasEstimate(
		orderId: string,
	): { fillGas: bigint; postGas: bigint; relayerFeeInFeeToken: bigint; relayerFeeInNativeToken: bigint } | null {
		try {
			const cache = this.cacheData.gasEstimates[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					fillGas: BigInt(cache.fillGas),
					postGas: BigInt(cache.postGas),
					relayerFeeInFeeToken: BigInt(cache.relayerFeeInFeeToken),
					relayerFeeInNativeToken: BigInt(cache.relayerFeeInNativeToken),
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting gas estimate")
			return null
		}
	}

	setGasEstimate(
		orderId: string,
		fillGas: bigint,
		postGas: bigint,
		relayerFeeInFeeToken: bigint,
		relayerFeeInNativeToken: bigint,
	): void {
		if (fillGas <= 0n || postGas <= 0n) {
			throw new Error("Gas values must be positive")
		}
		try {
			this.cleanupStaleData()
			this.cacheData.gasEstimates[orderId] = {
				fillGas: fillGas.toString(),
				postGas: postGas.toString(),
				relayerFeeInFeeToken: relayerFeeInFeeToken.toString(),
				relayerFeeInNativeToken: relayerFeeInNativeToken.toString(),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting gas estimate")
			throw error
		}
	}

	getSwapOperations(orderId: string): { calls: SwapCall[]; totalGasEstimate: bigint } | null {
		try {
			const cache = this.cacheData.swapOperations[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					calls: cache.calls,
					totalGasEstimate: BigInt(cache.totalGasEstimate),
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting swap operations")
			return null
		}
	}

	setSwapOperations(orderId: string, calls: SwapCall[], totalGasEstimate: bigint): void {
		try {
			this.cleanupStaleData()
			this.cacheData.swapOperations[orderId] = {
				calls,
				totalGasEstimate: totalGasEstimate.toString(),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting swap operations")
			throw error
		}
	}

	getFeeTokenWithDecimals(chain: string): { address: HexString; decimals: number } | null {
		try {
			const cache = this.cacheData.feeTokens[chain]
			if (cache) {
				return {
					address: cache.address,
					decimals: cache.decimals,
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting fee token with decimals")
			return null
		}
	}

	setFeeTokenWithDecimals(chain: string, address: HexString, decimals: number): void {
		try {
			this.cleanupStaleData()

			if (!this.cacheData.feeTokens[chain]) {
				this.cacheData.feeTokens[chain] = { address, decimals }
			} else {
				this.cacheData.feeTokens[chain] = { address, decimals }
			}
		} catch (error) {
			this.logger.error({ chain: chain, err: error }, "Error setting fee token with decimals")
			throw error
		}
	}

	getPerByteFee(sourceChain: string, destChain: string): bigint | null {
		try {
			const sourceMap = this.cacheData.perByteFees[sourceChain]
			if (sourceMap && sourceMap[destChain]) {
				return sourceMap[destChain]
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting per byte fee")
			return null
		}
	}

	setPerByteFee(sourceChain: string, destChain: string, perByteFee: bigint): void {
		try {
			this.cleanupStaleData()

			if (!this.cacheData.perByteFees[sourceChain]) {
				this.cacheData.perByteFees[sourceChain] = {}
			}
			this.cacheData.perByteFees[sourceChain][destChain] = perByteFee
		} catch (error) {
			this.logger.error(
				{ sourceChain: sourceChain, destChain: destChain, err: error },
				"Error setting per byte fee",
			)
			throw error
		}
	}

	getTokenDecimals(chain: string, tokenAddress: HexString): number | null {
		try {
			const chainCache = this.cacheData.tokenDecimals[chain]
			if (chainCache && chainCache[tokenAddress]) {
				return chainCache[tokenAddress]
			}
			return null
		} catch {
			return null
		}
	}

	setTokenDecimals(chain: string, tokenAddress: HexString, decimals: number): void {
		try {
			this.cleanupStaleData()
			// Ensure the chain object exists before setting the token decimals
			if (!this.cacheData.tokenDecimals[chain]) {
				this.cacheData.tokenDecimals[chain] = {}
			}
			this.cacheData.tokenDecimals[chain][tokenAddress] = decimals
		} catch (error) {
			this.logger.error({ chain: chain, tokenAddress: tokenAddress, err: error }, "Error setting token decimals")
			throw error
		}
	}
}
