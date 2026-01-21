import { toHex } from "viem"
import type { ChainConfig, HexString } from "@/types"
import { chainConfigs, getConfigByStateMachineId, Chains, hyperbridgeAddress } from "@/configs/chain"

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

	getUsdtAsset(chain: string): HexString {
		return (this.getConfig(chain)?.assets?.USDT ?? "0x") as HexString
	}

	getUsdcAsset(chain: string): HexString {
		return (this.getConfig(chain)?.assets?.USDC ?? "0x") as HexString
	}

	getChainId(chain: string): number {
		return this.getConfig(chain)?.chainId ?? 0
	}

	getConsensusStateId(chain: string): HexString {
		const id = this.getConfig(chain)?.consensusStateId
		return id ? toHex(id) : ("0x" as HexString)
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

	getPermit2Address(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.Permit2 ?? "0x") as HexString
	}

	getCoingeckoId(chain: string): string | undefined {
		return this.getConfig(chain)?.coingeckoId
	}

	getEtherscanApiKey(): string | undefined {
		return typeof process !== "undefined" ? (process as any)?.env?.ETHERSCAN_API_KEY : undefined
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

	getIntentGatewayV2Address(chain: string): HexString {
		return (this.getConfig(chain)?.addresses.IntentGatewayV2 ?? "0x") as HexString
	}

	getHyperbridgeAddress(): string {
		return hyperbridgeAddress
	}
}
