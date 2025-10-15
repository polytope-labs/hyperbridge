import { toHex } from "viem"
import type { ChainConfig, HexString } from "@/types"
import {
	addresses,
	assets,
	chainIds,
	consensusStateIds,
	coingeckoIds,
	Chains,
	WrappedNativeDecimals,
	createRpcUrls,
} from "@/configs/chain"

export class ChainConfigService {
	private rpcUrls: Record<Chains, string>

	constructor(env: NodeJS.ProcessEnv = process.env) {
		this.rpcUrls = createRpcUrls(env)
	}

	getChainConfig(chain: string): ChainConfig {
		return {
			chainId: chainIds[chain as keyof typeof chainIds],
			rpcUrl: this.rpcUrls[chain as Chains],
			intentGatewayAddress: addresses.IntentGateway[chain as keyof typeof addresses.IntentGateway]!,
		}
	}

	getIntentGatewayAddress(chain: string): `0x${string}` {
		return addresses.IntentGateway[chain as keyof typeof addresses.IntentGateway]! as `0x${string}`
	}

	getHostAddress(chain: string): `0x${string}` {
		return addresses.Host[chain as keyof typeof addresses.Host]! as `0x${string}`
	}

	getWrappedNativeAssetWithDecimals(chain: string): { asset: HexString; decimals: number } {
		return {
			asset: assets[chain as keyof typeof assets].WETH as HexString,
			decimals: WrappedNativeDecimals[chain as keyof typeof WrappedNativeDecimals],
		}
	}

	getDaiAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].DAI as HexString
	}

	getUsdtAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].USDT as HexString
	}

	getUsdcAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].USDC as HexString
	}

	getChainId(chain: string): number {
		return chainIds[chain as keyof typeof chainIds]
	}

	getConsensusStateId(chain: string): HexString {
		return toHex(consensusStateIds[chain as keyof typeof consensusStateIds])
	}

	getHyperbridgeChainId(): number {
		return chainIds[Chains.HYPERBRIDGE_GARGANTUA]
	}

	getRpcUrl(chain: string): string {
		return this.rpcUrls[chain as Chains]
	}

	getUniswapRouterV2Address(chain: string): HexString {
		return addresses.UniswapRouter02[chain as keyof typeof addresses.UniswapRouter02]! as HexString
	}

	getUniswapV2FactoryAddress(chain: string): HexString {
		return addresses.UniswapV2Factory[chain as keyof typeof addresses.UniswapV2Factory]! as HexString
	}

	getUniversalRouterAddress(chain: string): HexString {
		return addresses.UniversalRouter[chain as keyof typeof addresses.UniversalRouter]! as HexString
	}

	getUniswapV3QuoterAddress(chain: string): HexString {
		return addresses.UniswapV3Quoter[chain as keyof typeof addresses.UniswapV3Quoter]! as HexString
	}

	getUniswapV4QuoterAddress(chain: string): HexString {
		return addresses.UniswapV4Quoter[chain as keyof typeof addresses.UniswapV4Quoter]! as HexString
	}

	getPermit2Address(chain: string): HexString {
		return addresses.Permit2[chain as keyof typeof addresses.Permit2]! as HexString
	}

	getCoingeckoId(chain: string): string | undefined {
		return coingeckoIds[chain as keyof typeof coingeckoIds]
	}
}
