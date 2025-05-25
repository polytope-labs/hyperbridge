import { toHex } from "viem"
import { ChainConfig, HexString } from "hyperbridge-sdk"
import {
	addresses,
	assets,
	chainIds,
	consensusStateIds,
	Chains,
	WrappedNativeDecimals,
	createRpcUrls,
} from "@/config/chain"

/**
 * Centralizes access to chain configuration
 */
export class ChainConfigService {
	private rpcUrls: Record<Chains, string>

	constructor(env: NodeJS.ProcessEnv = process.env) {
		this.rpcUrls = createRpcUrls(env)
	}

	/**
	 * Gets the chain configuration for a given chain
	 */
	getChainConfig(chain: string): ChainConfig {
		return {
			chainId: chainIds[chain as keyof typeof chainIds],
			rpcUrl: this.rpcUrls[chain as Chains],
			intentGatewayAddress: addresses.IntentGateway[chain as keyof typeof addresses.IntentGateway]!,
		}
	}

	/**
	 * Gets the IntentGateway address for a given chain
	 */
	getIntentGatewayAddress(chain: string): `0x${string}` {
		return addresses.IntentGateway[chain as keyof typeof addresses.IntentGateway]! as `0x${string}`
	}

	/**
	 * Gets the Host address for a given chain
	 */
	getHostAddress(chain: string): `0x${string}` {
		return addresses.Host[chain as keyof typeof addresses.Host]! as `0x${string}`
	}

	/**
	 * Gets the Native asset for a given chain
	 */
	getWrappedNativeAssetWithDecimals(chain: string): { asset: HexString; decimals: number } {
		return {
			asset: assets[chain as keyof typeof assets].WETH as HexString,
			decimals: WrappedNativeDecimals[chain as keyof typeof WrappedNativeDecimals],
		}
	}

	/**
	 * Gets the DAI asset for a given chain
	 */
	getDaiAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].DAI as HexString
	}

	/**
	 * Gets the USDT asset for a given chain
	 */
	getUsdtAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].USDT as HexString
	}

	/**
	 * Gets the USDC asset for a given chain
	 */
	getUsdcAsset(chain: string): HexString {
		return assets[chain as keyof typeof assets].USDC as HexString
	}

	/**
	 * Gets the chain ID for a given chain
	 */
	getChainId(chain: string): number {
		return chainIds[chain as keyof typeof chainIds]
	}

	/**
	 * Gets the consensus state ID for a given chain
	 */
	getConsensusStateId(chain: string): HexString {
		return toHex(consensusStateIds[chain as keyof typeof consensusStateIds])
	}

	/**
	 * Gets the Hyperbridge Gargantua chain ID
	 */
	getHyperbridgeChainId(): number {
		return chainIds[Chains.HYPERBRIDGE_GARGANTUA]
	}

	/**
	 * Gets the RPC URL for a given chain
	 */
	getRpcUrl(chain: string): string {
		return this.rpcUrls[chain as Chains]
	}

	/**
	 * Gets the Uniswap Router V2 contract address for a given chain
	 */
	getUniswapRouterV2Address(chain: string): HexString {
		return addresses.UniswapRouter02[chain as keyof typeof addresses.UniswapRouter02]! as HexString
	}

	/**
	 * Gets the Uniswap V2 Factory contract address for a given chain
	 */
	getUniswapV2FactoryAddress(chain: string): HexString {
		return addresses.UniswapV2Factory[chain as keyof typeof addresses.UniswapV2Factory]! as HexString
	}

	/**
	 * Gets the Batch Executor contract address for a given chain
	 */
	getBatchExecutorAddress(chain: string): HexString {
		return addresses.BatchExecutor[chain as keyof typeof addresses.BatchExecutor]! as HexString
	}

	/**
	 * Gets the Universal Router contract address for a given chain
	 */
	getUniversalRouterAddress(chain: string): HexString {
		return addresses.UniversalRouter[chain as keyof typeof addresses.UniversalRouter]! as HexString
	}

	/**
	 * Gets the Uniswap V3 Router contract address for a given chain
	 */
	getUniswapV3RouterAddress(chain: string): HexString {
		return addresses.UniswapV3Router[chain as keyof typeof addresses.UniswapV3Router]! as HexString
	}

	/**
	 * Gets the Uniswap V3 Factory contract address for a given chain
	 */
	getUniswapV3FactoryAddress(chain: string): HexString {
		return addresses.UniswapV3Factory[chain as keyof typeof addresses.UniswapV3Factory]! as HexString
	}

	/**
	 * Gets the Uniswap V3 Quoter contract address for a given chain
	 */
	getUniswapV3QuoterAddress(chain: string): HexString {
		return addresses.UniswapV3Quoter[chain as keyof typeof addresses.UniswapV3Quoter]! as HexString
	}
}
