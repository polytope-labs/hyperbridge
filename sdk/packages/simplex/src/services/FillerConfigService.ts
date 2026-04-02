import type { ChainConfig, HexString } from "@hyperbridge/sdk"
import { ChainConfigService } from "@hyperbridge/sdk"
import { LogLevel } from "./Logger"

export interface UserProvidedChainConfig {
	rpcUrl: string
	bundlerUrl: string
}

export interface ResolvedChainConfig {
	chainId: number
	rpcUrl: string
	bundlerUrl?: string
}

/**
 * Fetches the chain ID from an RPC endpoint using eth_chainId.
 */
export async function fetchChainId(rpcUrl: string): Promise<number> {
	const response = await fetch(rpcUrl, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", method: "eth_chainId", params: [], id: 1 }),
	})
	if (!response.ok) {
		throw new Error(`Failed to fetch chainId from ${rpcUrl}: HTTP ${response.status}`)
	}
	const json = (await response.json()) as { result?: string; error?: { message: string } }
	if (json.error) {
		throw new Error(`eth_chainId error from ${rpcUrl}: ${json.error.message}`)
	}
	return Number(json.result)
}

/**
 * Resolves chain IDs for all user-provided chain configs by querying each RPC.
 */
export async function resolveChainConfigs(chains: UserProvidedChainConfig[]): Promise<ResolvedChainConfig[]> {
	return Promise.all(
		chains.map(async (chain) => {
			const chainId = await fetchChainId(chain.rpcUrl)
			return { chainId, rpcUrl: chain.rpcUrl, bundlerUrl: chain.bundlerUrl }
		}),
	)
}


export interface GasFeeBumpConfig {
	maxPriorityFeePerGasBumpPercent?: number
	maxFeePerGasBumpPercent?: number
}

export interface RebalancingConfig {
	triggerPercentage: number
	baseBalances: {
		USDC?: Record<string, string>
		USDT?: Record<string, string>
	}
}

export interface FillerConfig {
	maxConcurrentOrders: number
	logging?: LogLevel
	hyperbridgeWsUrl?: string
	substratePrivateKey?: string
	entryPointAddress?: string
	dataDir?: string
	/**
	 * Optional gas fee bump configuration for UserOperation gas estimation.
	 * If not provided, defaults will be used (8% for priority fee, 10% for max fee).
	 */
	gasFeeBump?: GasFeeBumpConfig
	rebalancing?: RebalancingConfig
	/**
	 * Target gas units the EntryPoint deposit should cover per chain.
	 * Defaults to 3,000,000 if not set.
	 */
	targetGasUnits?: number
}

/**
 * Simplified configuration service for the filler that wraps ChainConfigService
 * and only requires minimal user configuration (RPC URLs, private keys, etc.)
 */
export class FillerConfigService {
	private chainConfigService: ChainConfigService
	private rpcOverrides: Map<number, string> = new Map()
	private bundlerUrls: Map<number, string> = new Map()
	private fillerConfig?: FillerConfig

	constructor(chainConfigs: ResolvedChainConfig[], fillerConfig?: FillerConfig) {
		chainConfigs.forEach((config) => {
			if (config.rpcUrl) {
				this.rpcOverrides.set(config.chainId, config.rpcUrl)
			}
			if (config.bundlerUrl) {
				this.bundlerUrls.set(config.chainId, config.bundlerUrl)
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

	getUsdcDecimals(chain: string): number {
		return this.chainConfigService.getUsdcDecimals(chain)
	}

	getUsdtDecimals(chain: string): number {
		return this.chainConfigService.getUsdtDecimals(chain)
	}

	getChainId(chain: string): number {
		return this.chainConfigService.getChainId(chain)
	}

	getConsensusStateId(chain: string): string {
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
		const raw = chain.includes("EVM") ? chain.slice(4) : chain
		const id = Number.parseInt(raw, 10)
		if (Number.isNaN(id)) {
			throw new Error(`Cannot derive chain ID from state machine ID: "${chain}"`)
		}
		return id
	}

	getUniswapRouterV2Address(chain: string): HexString {
		return this.chainConfigService.getUniswapRouterV2Address(chain)
	}

	getAerodromeRouterAddress(chain: string): HexString {
		return this.chainConfigService.getAerodromeRouterAddress(chain)
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

	getUniswapV4PositionManagerAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV4PositionManagerAddress(chain)
	}

	getUniswapV4PoolManagerAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV4PoolManagerAddress(chain)
	}

	getUniswapV4StateViewAddress(chain: string): HexString {
		return this.chainConfigService.getUniswapV4StateViewAddress(chain)
	}

	getPermit2Address(chain: string): HexString {
		return this.chainConfigService.getPermit2Address(chain)
	}

	getCoingeckoId(chain: string): string | undefined {
		return this.chainConfigService.getCoingeckoId(chain)
	}

	getCNgnAsset(chain: string): HexString | undefined {
		return this.chainConfigService.getCNgnAsset(chain)
	}

	getCNgnDecimals(chain: string): number | undefined {
		return this.chainConfigService.getCNgnDecimals(chain)
	}

	getExtAsset(chain: string): HexString | undefined {
		return this.chainConfigService.getExtAsset(chain)
	}

	getExtDecimals(chain: string): number | undefined {
		return this.chainConfigService.getExtDecimals(chain)
	}

	getConfiguredChainIds(): number[] {
		return Array.from(this.rpcOverrides.keys())
	}

	getLoggingConfig(): LogLevel | undefined {
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

	getSolverAccountContractAddress(chain: string): HexString | undefined {
		return this.chainConfigService.getSolverAccountAddress(chain) as HexString | undefined
	}

	getDataDir(): string | undefined {
		return this.fillerConfig?.dataDir
	}

	getBundlerUrl(chain: string): string | undefined {
		const chainId = this.getChainIdFromStateMachineId(chain)
		return this.bundlerUrls.get(chainId)
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

	/**
	 * Get the LayerZero Endpoint ID for the chain
	 * Used for USDT0 cross-chain transfers via LayerZero OFT
	 */
	getLayerZeroEid(chain: string): number | undefined {
		return this.chainConfigService.getLayerZeroEid(chain)
	}

	/**
	 * Get the USDT0 OFT contract address for the chain
	 * On Ethereum: OFT Adapter (locks/unlocks USDT)
	 * On other chains: OFT contract (mints/burns USDT0)
	 */
	getUsdt0OftAddress(chain: string): HexString | undefined {
		return this.chainConfigService.getUsdt0OftAddress(chain)
	}

	/**
	 * Get rebalancing configuration
	 */
	getRebalancingConfig(): RebalancingConfig | undefined {
		return this.fillerConfig?.rebalancing
	}

	/**
	 * Get base balance for a specific chain and asset
	 * @param chainId Chain ID as number
	 * @param asset "USDC" or "USDT"
	 * @returns Base balance as Decimal, or undefined if not configured
	 */
	getBaseBalance(chainId: number, asset: "USDC" | "USDT"): number | undefined {
		const rebalancingConfig = this.fillerConfig?.rebalancing
		if (!rebalancingConfig) {
			return undefined
		}

		const chainIdStr = chainId.toString()
		const baseBalances = rebalancingConfig.baseBalances[asset]
		if (!baseBalances || !baseBalances[chainIdStr]) {
			return undefined
		}

		return Number.parseFloat(baseBalances[chainIdStr])
	}

	/**
	 * Get trigger percentage for rebalancing
	 * @returns Trigger percentage (0-1), or undefined if not configured
	 */
	getTriggerPercentage(): number | undefined {
		return this.fillerConfig?.rebalancing?.triggerPercentage
	}

	/**
	 * Get target gas units for EntryPoint deposits.
	 * Defaults to 3,000,000 if not configured.
	 */
	getTargetGasUnits(): bigint {
		return BigInt(this.fillerConfig?.targetGasUnits ?? 3_000_000)
	}
}
