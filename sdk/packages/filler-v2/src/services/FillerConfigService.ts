import type { ChainConfig, HexString } from "@hyperbridge/sdk"
import { ChainConfigService } from "@hyperbridge/sdk"
import { LogLevel } from "./Logger"

export interface UserProvidedChainConfig {
	chainId: number
	rpcUrl: string
}

export interface LoggingConfig {
	level?: LogLevel
}

export interface GasFeeBumpConfig {
	maxPriorityFeePerGasBumpPercent?: number
	maxFeePerGasBumpPercent?: number
}

export interface FillerConfig {
	privateKey: string
	maxConcurrentOrders: number
	logging?: LoggingConfig
	hyperbridgeWsUrl?: string
	substratePrivateKey?: string
	entryPointAddress?: string
	solverAccountContractAddress?: string
	dataDir?: string
	bundlerUrl?: string
	/**
	 * Optional gas fee bump configuration for UserOperation gas estimation.
	 * If not provided, defaults will be used (8% for priority fee, 10% for max fee).
	 */
	gasFeeBump?: GasFeeBumpConfig
}

/**
 * Simplified configuration service for the filler that wraps ChainConfigService
 * and only requires minimal user configuration (RPC URLs, private keys, etc.)
 */
export class FillerConfigService {
	private chainConfigService: ChainConfigService
	private rpcOverrides: Map<number, string> = new Map()
	private fillerConfig?: FillerConfig

	constructor(chainConfigs: UserProvidedChainConfig[], fillerConfig?: FillerConfig) {
		chainConfigs.forEach((config) => {
			if (config.rpcUrl) {
				this.rpcOverrides.set(config.chainId, config.rpcUrl)
			}
		})

		this.chainConfigService = new ChainConfigService({})
		this.fillerConfig = fillerConfig
	}

	getChainConfig(chain: string): ChainConfig {
		const baseConfig = this.chainConfigService.getChainConfig(chain)

		// Override RPC URL if user provided a custom one
		const customRpcUrl = this.getRpcUrl(chain)

		return {
			...baseConfig,
			rpcUrl: customRpcUrl,
		}
	}

	getIntentGatewayV2Address(chain: string): `0x${string}` {
		return this.chainConfigService.getIntentGatewayV2Address(chain)
	}

	getHostAddress(chain: string): `0x${string}` {
		return this.chainConfigService.getHostAddress(chain)
	}

	getWrappedNativeAssetWithDecimals(chain: string): { asset: HexString; decimals: number } {
		return this.chainConfigService.getWrappedNativeAssetWithDecimals(chain)
	}

	getDaiAsset(chain: string): HexString {
		return this.chainConfigService.getDaiAsset(chain)
	}

	getUsdtAsset(chain: string): HexString {
		return this.chainConfigService.getUsdtAsset(chain)
	}

	getUsdcAsset(chain: string): HexString {
		return this.chainConfigService.getUsdcAsset(chain)
	}

	getChainId(chain: string): number {
		return this.chainConfigService.getChainId(chain)
	}

	getConsensusStateId(chain: string): HexString {
		return this.chainConfigService.getConsensusStateId(chain)
	}

	getHyperbridgeChainId(): number {
		// Use SDK's default Hyperbridge chain ID
		return this.chainConfigService.getHyperbridgeChainId()
	}

	getHyperbridgeRpcUrl(): string {
		// Use SDK's default Hyperbridge RPC URL
		return this.chainConfigService.getRpcUrl("KUSAMA-4009")
	}

	getRpcUrl(chain: string): string {
		const chainId = this.getChainIdFromStateMachineId(chain)
		const customRpcUrl = this.rpcOverrides.get(chainId)
		if (customRpcUrl) {
			return customRpcUrl
		}

		// Fall back to SDK's default RPC URL
		return this.chainConfigService.getRpcUrl(chain)
	}

	private getChainIdFromStateMachineId(chain: string): number {
		if (chain.includes("EVM")) {
			return Number.parseInt(chain.slice(4))
		}

		return Number.parseInt(chain)
	}

	getUniswapRouterV2Address(chain: string): HexString {
		return this.chainConfigService.getUniswapRouterV2Address(chain)
	}

	getUniswapV2FactoryAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV2FactoryAddress(chain)
	}

	getUniversalRouterAddress(chain: string): HexString {
		return this.chainConfigService.getUniversalRouterAddress(chain)
	}

	getUniswapV3QuoterAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV3QuoterAddress(chain)
	}

	getUniswapV4QuoterAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV4QuoterAddress(chain)
	}

	getPermit2Address(chain: string): HexString {
		return this.chainConfigService.getPermit2Address(chain)
	}

	getCoingeckoId(chain: string): string | undefined {
		return this.chainConfigService.getCoingeckoId(chain)
	}

	getConfiguredChainIds(): number[] {
		return Array.from(this.rpcOverrides.keys())
	}

	getLoggingConfig(): LoggingConfig | undefined {
		return this.fillerConfig?.logging
	}

	getHyperbridgeAddress(): string {
		return this.chainConfigService.getHyperbridgeAddress()
	}

	getHyperbridgeWsUrl(): string | undefined {
		return this.fillerConfig?.hyperbridgeWsUrl
	}

	getSubstratePrivateKey(): string | undefined {
		return this.fillerConfig?.substratePrivateKey
	}

	getEntryPointAddress(chain: string): HexString | undefined {
		return this.chainConfigService.getEntryPointV08Address(chain) as HexString | undefined
	}

	getSolverAccountContractAddress(): HexString | undefined {
		return this.fillerConfig?.solverAccountContractAddress as HexString | undefined
	}

	getDataDir(): string | undefined {
		return this.fillerConfig?.dataDir
	}

	getBundlerUrl(): string | undefined {
		return this.fillerConfig?.bundlerUrl
	}

	/**
	 * Get the maxPriorityFeePerGas bump percentage.
	 * @returns The configured percentage or undefined if not set (default 8% will be used)
	 */
	getMaxPriorityFeePerGasBumpPercent(): number | undefined {
		return this.fillerConfig?.gasFeeBump?.maxPriorityFeePerGasBumpPercent
	}

	/**
	 * Get the maxFeePerGas bump percentage.
	 * @returns The configured percentage or undefined if not set (default 10% will be used)
	 */
	getMaxFeePerGasBumpPercent(): number | undefined {
		return this.fillerConfig?.gasFeeBump?.maxFeePerGasBumpPercent
	}

	/**
	 * Get the full gas fee bump configuration.
	 * @returns The gas fee bump config or undefined if not set
	 */
	getGasFeeBumpConfig(): GasFeeBumpConfig | undefined {
		return this.fillerConfig?.gasFeeBump
	}
}
