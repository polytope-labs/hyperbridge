import type { ChainConfig, HexString } from "@/types"
import {
	chainConfigs,
	getConfigByStateMachineId,
	type Chains,
	hyperbridgeAddress,
	type ConfiguredAssetSymbol,
	type UniswapV4PoolConfigData,
} from "@/configs/chain"

export class ChainConfigService {
	private rpcUrls: Record<string, string> = {}

	constructor(env: NodeJS.ProcessEnv = process.env) {
		for (const config of Object.values(chainConfigs)) {
			if (config.rpcEnvKey) {
				this.rpcUrls[config.stateMachineId] = env[config.rpcEnvKey] || config.defaultRpcUrl || ""
			}
		}
	}

	private getConfig(chain: string) {
		return getConfigByStateMachineId(chain as Chains)
	}

	getChainConfig(chain: string): ChainConfig {
		const config = this.getConfig(chain)
		return {
			chainId: config?.chainId ?? 0,
			rpcUrl: this.rpcUrls[chain] ?? "",
			intentGatewayAddress: config?.addresses.IntentGateway ?? ("0x" as `0x${string}`),
		}
	}

	getIntentGatewayAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.IntentGateway ?? "0x") as HexString
	}

	getTokenGatewayAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.TokenGateway ?? "0x") as HexString
	}

	getHostAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.Host ?? "0x") as HexString
	}

	getWrappedNativeAssetWithDecimals(chain: string): { asset: HexString; decimals: number } {
		const config = this.getConfig(chain)
		return {
			asset: (config?.assets?.WETH ?? "0x") as HexString,
			decimals: config?.wrappedNativeDecimals ?? 18,
		}
	}

	getDaiAsset(chain: string): HexString {
		return (this.getConfig(chain)?.assets?.DAI ?? "0x") as HexString
	}

	getAssetAddress(chain: string, symbol: ConfiguredAssetSymbol): HexString | undefined {
		return this.getConfig(chain)?.assets?.[symbol] as HexString | undefined
	}

	/**
	 * Resolves configured token metadata from an address on a specific chain.
	 * This is used by SDK helpers that accept token addresses rather than caller-
	 * supplied symbols or decimals.
	 */
	getAssetMetadataByAddress(
		chain: string,
		address: HexString,
	): { symbol: ConfiguredAssetSymbol; address: HexString; decimals?: number } | undefined {
		const config = this.getConfig(chain)
		if (!config?.assets) return

		const normalizedAddress = address.toLowerCase()
		for (const [symbol, configuredAddress] of Object.entries(config.assets)) {
			if (configuredAddress.toLowerCase() !== normalizedAddress) continue
			return {
				symbol: symbol as ConfiguredAssetSymbol,
				address: configuredAddress as HexString,
				decimals: config.tokenDecimals?.[symbol as keyof typeof config.tokenDecimals],
			}
		}
	}

	getUsdtAsset(chain: string): HexString {
		return (this.getConfig(chain)?.assets?.USDT ?? "0x") as HexString
	}

	getUsdcAsset(chain: string): HexString {
		return (this.getConfig(chain)?.assets?.USDC ?? "0x") as HexString
	}

	getUsdcDecimals(chain: string): number {
		return this.getConfig(chain)?.tokenDecimals?.USDC!
	}

	getUsdtDecimals(chain: string): number {
		return this.getConfig(chain)?.tokenDecimals?.USDT!
	}

	getCNgnAsset(chain: string): HexString | undefined {
		return this.getConfig(chain)?.assets?.cNGN as HexString | undefined
	}

	getCNgnDecimals(chain: string): number | undefined {
		return this.getConfig(chain)?.tokenDecimals?.cNGN
	}

	getExtAsset(chain: string): HexString | undefined {
		return this.getConfig(chain)?.assets?.EXT as HexString | undefined
	}

	getExtDecimals(chain: string): number | undefined {
		return this.getConfig(chain)?.tokenDecimals?.EXT
	}

	getChainId(chain: string): number {
		return this.getConfig(chain)?.chainId ?? 0
	}

	getConsensusStateId(chain: string): string {
		const id = this.getConfig(chain)?.consensusStateId
		if (!id) throw new Error(`No consensusStateId configured for chain: ${chain}`)
		return id
	}

	getHyperbridgeChainId(): number {
		return chainConfigs[4009]?.chainId ?? 4009
	}

	getRpcUrl(chain: string): string {
		return this.rpcUrls[chain] ?? ""
	}

	getUniswapRouterV2Address(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapRouter02 ?? "0x") as HexString
	}

	getAerodromeRouterAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.AerodromeRouter ?? "0x") as HexString
	}

	getUniswapV2FactoryAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV2Factory ?? "0x") as HexString
	}

	getUniswapV3FactoryAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV3Factory ?? "0x") as HexString
	}

	getUniversalRouterAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniversalRouter ?? "0x") as HexString
	}

	getUniswapV3QuoterAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV3Quoter ?? "0x") as HexString
	}

	getUniswapV4QuoterAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV4Quoter ?? "0x") as HexString
	}

	getUniswapV4PositionManagerAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV4PositionManager ?? "0x") as HexString
	}

	getUniswapV4PoolManagerAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV4PoolManager ?? "0x") as HexString
	}

	getUniswapV4StateViewAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.UniswapV4StateView ?? "0x") as HexString
	}

	getUniswapV4PoolConfigs(chain: string): UniswapV4PoolConfigData[] {
		return this.getConfig(chain)?.uniswapV4Pools ?? []
	}

	getPermit2Address(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.Permit2 ?? "0x") as HexString
	}

	getSolverAccountAddress(chain: string): HexString | undefined {
		return this.getConfig(chain)?.addresses.SolverAccount as HexString | undefined
	}

	getCoingeckoId(chain: string): string | undefined {
		return this.getConfig(chain)?.coingeckoId
	}

	getEtherscanApiKey(): string | undefined {
		return typeof process !== "undefined" ? process.env?.ETHERSCAN_API_KEY : undefined
	}

	getCalldispatcherAddress(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.Calldispatcher ?? "0x") as HexString
	}

	getTokenStorageSlots(
		chain: string,
		tokenAddress: string,
	): { balanceSlot: number; allowanceSlot: number } | undefined {
		const config = this.getConfig(chain)
		if (!config?.tokenStorageSlots || !config?.assets) return undefined

		const normalized = tokenAddress.toLowerCase()
		for (const [symbol, address] of Object.entries(config.assets)) {
			if (address.toLowerCase() === normalized) {
				return config.tokenStorageSlots[symbol as keyof typeof config.tokenStorageSlots]
			}
		}
		return undefined
	}

	getPopularTokens(chain: string): string[] {
		return this.getConfig(chain)?.popularTokens ?? []
	}

	getEntryPointV08Address(chain: string): HexString {
		return this.getConfig(chain)?.addresses.EntryPointV08!
	}

	getCirclePaymasterAddress(chain: string): HexString | undefined {
		return this.getConfig(chain)?.addresses.CirclePaymaster as HexString | undefined
	}

	getHyperbridgeAddress(): string {
		return hyperbridgeAddress
	}

	/**
	 * Get the LayerZero Endpoint ID for the chain
	 * Used for USDT0 cross-chain transfers via LayerZero OFT
	 */
	getLayerZeroEid(chain: string): number | undefined {
		return this.getConfig(chain)?.layerZeroEid
	}

	/**
	 * Get the USDT0 OFT contract address for the chain
	 * On Ethereum: OFT Adapter (locks/unlocks USDT)
	 * On other chains: OFT contract (mints/burns USDT0)
	 */
	getUsdt0OftAddress(chain: string): HexString | undefined {
		return this.getConfig(chain)?.addresses.Usdt0Oft as HexString | undefined
	}
}
