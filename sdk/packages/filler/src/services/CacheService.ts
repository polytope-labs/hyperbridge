interface GasEstimateCache {
	fillGas: string
	postGas: string
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
}

export class CacheService {
	private cacheData: CacheData
	private readonly CACHE_EXPIRY_MS = 1 * 60 * 1000 // 1 minute

	constructor() {
		this.cacheData = { gasEstimates: {}, swapOperations: {} }
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

	getGasEstimate(orderId: string): { fillGas: bigint; postGas: bigint } | null {
		try {
			const cache = this.cacheData.gasEstimates[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					fillGas: BigInt(cache.fillGas),
					postGas: BigInt(cache.postGas),
				}
			}
			return null
		} catch (error) {
			console.error("Error getting gas estimate:", error)
			return null
		}
	}

	setGasEstimate(orderId: string, fillGas: bigint, postGas: bigint): void {
		if (fillGas <= 0n || postGas <= 0n) {
			throw new Error("Gas values must be positive")
		}
		try {
			this.cleanupStaleData()
			this.cacheData.gasEstimates[orderId] = {
				fillGas: fillGas.toString(),
				postGas: postGas.toString(),
				timestamp: Date.now(),
			}
		} catch (error) {
			console.error("Error setting gas estimate:", error)
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
			console.error("Error getting swap operations:", error)
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
			console.error("Error setting swap operations:", error)
			throw error
		}
	}
}
