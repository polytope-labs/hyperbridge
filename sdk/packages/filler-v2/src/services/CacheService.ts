import { HexString } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"

interface GasEstimateCache {
	totalCostInSourceFeeToken: string
	dispatchFee: string
	nativeDispatchFee: string
	callGasLimit: string
	verificationGasLimit: string
	preVerificationGas: string
	maxFeePerGas: string
	maxPriorityFeePerGas: string
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

interface FillerOutputCache {
	token: HexString
	amount: string
}

interface FillerOutputsCache {
	outputs: FillerOutputCache[]
	timestamp: number
}

interface CacheData {
	gasEstimates: Record<string, GasEstimateCache>
	swapOperations: Record<string, SwapOperationsCache>
	fillerOutputs: Record<string, FillerOutputsCache>
	feeTokens: Record<string, { address: HexString; decimals: number }>
	perByteFees: Record<string, Record<string, bigint>>
	tokenDecimals: Record<string, Record<HexString, number>>
	solverSelection: Record<string, boolean>
}

export class CacheService {
	private cacheData: CacheData
	private readonly CACHE_EXPIRY_MS = 1 * 60 * 1000 // 1 minute
	private logger = getLogger("cache-service")

	constructor() {
		this.cacheData = {
			gasEstimates: {},
			swapOperations: {},
			fillerOutputs: {},
			feeTokens: {},
			perByteFees: {},
			tokenDecimals: {},
			solverSelection: {},
		}
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

		// Clean up filler outputs
		const staleFillerOutputIds = Object.entries(this.cacheData.fillerOutputs)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleFillerOutputIds.forEach((orderId) => {
			delete this.cacheData.fillerOutputs[orderId]
		})
	}

	getGasEstimate(orderId: string): {
		totalCostInSourceFeeToken: bigint
		dispatchFee: bigint
		nativeDispatchFee: bigint
		callGasLimit: bigint
		verificationGasLimit: bigint
		preVerificationGas: bigint
		maxFeePerGas: bigint
		maxPriorityFeePerGas: bigint
	} | null {
		try {
			const cache = this.cacheData.gasEstimates[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					totalCostInSourceFeeToken: BigInt(cache.totalCostInSourceFeeToken),
					dispatchFee: BigInt(cache.dispatchFee),
					nativeDispatchFee: BigInt(cache.nativeDispatchFee),
					callGasLimit: BigInt(cache.callGasLimit),
					verificationGasLimit: BigInt(cache.verificationGasLimit),
					preVerificationGas: BigInt(cache.preVerificationGas),
					maxFeePerGas: BigInt(cache.maxFeePerGas),
					maxPriorityFeePerGas: BigInt(cache.maxPriorityFeePerGas),
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
		totalCostInSourceFeeToken: bigint,
		dispatchFee: bigint,
		nativeDispatchFee: bigint,
		callGasLimit: bigint,
		verificationGasLimit: bigint,
		preVerificationGas: bigint,
		maxFeePerGas: bigint,
		maxPriorityFeePerGas: bigint,
	): void {
		if (totalCostInSourceFeeToken <= 0n) {
			throw new Error("Total cost in source fee token must be positive")
		}
		try {
			this.cleanupStaleData()
			this.cacheData.gasEstimates[orderId] = {
				totalCostInSourceFeeToken: totalCostInSourceFeeToken.toString(),
				dispatchFee: dispatchFee.toString(),
				nativeDispatchFee: nativeDispatchFee.toString(),
				callGasLimit: callGasLimit.toString(),
				verificationGasLimit: verificationGasLimit.toString(),
				preVerificationGas: preVerificationGas.toString(),
				maxFeePerGas: maxFeePerGas.toString(),
				maxPriorityFeePerGas: maxPriorityFeePerGas.toString(),
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

	getFillerOutputs(orderId: string): { token: HexString; amount: bigint }[] | null {
		try {
			const cache = this.cacheData.fillerOutputs[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return cache.outputs.map((o) => ({
					token: o.token,
					amount: BigInt(o.amount),
				}))
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting filler outputs")
			return null
		}
	}

	setFillerOutputs(orderId: string, outputs: { token: HexString; amount: bigint }[]): void {
		try {
			this.cleanupStaleData()
			this.cacheData.fillerOutputs[orderId] = {
				outputs: outputs.map((o) => ({
					token: o.token,
					amount: o.amount.toString(),
				})),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting filler outputs")
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

	getSolverSelection(chain: string): boolean | null {
		const cached = this.cacheData.solverSelection[chain]
		return cached !== undefined ? cached : null
	}

	setSolverSelection(chain: string, active: boolean): void {
		this.cacheData.solverSelection[chain] = active
	}
}
