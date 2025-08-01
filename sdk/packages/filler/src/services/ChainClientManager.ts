import { PublicClient, WalletClient, createPublicClient, createWalletClient, http, type Chain } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { Order, HexString, ChainConfig } from "@hyperbridge/sdk"
import { ChainConfigService } from "./ChainConfigService"
import { viemChains } from "@/config/chain"

/**
 * Factory for creating and managing Viem clients
 */
export class ViemClientFactory {
	private publicClients: Map<number, PublicClient> = new Map()
	private walletClients: Map<number, WalletClient> = new Map()

	public getPublicClient(chainConfig: ChainConfig): PublicClient {
		if (!this.publicClients.has(chainConfig.chainId)) {
			const chain = viemChains[chainConfig.chainId]

			const publicClient = createPublicClient({
				chain,
				transport: http(chainConfig.rpcUrl, {
					timeout: 30000, // 30 seconds
					retryCount: 3,
					retryDelay: 1000,
				}),
			})

			this.publicClients.set(chainConfig.chainId, publicClient)
		}

		return this.publicClients.get(chainConfig.chainId)!
	}

	public getWalletClient(chainConfig: ChainConfig, privateKey: string): WalletClient {
		if (!this.walletClients.has(chainConfig.chainId)) {
			const chain = viemChains[chainConfig.chainId]
			const account = privateKeyToAccount(privateKey as `0x${string}`)

			const walletClient = createWalletClient({
				chain,
				account,
				transport: http(chainConfig.rpcUrl, {
					timeout: 30000,
					retryCount: 3,
					retryDelay: 1000,
				}),
			})

			this.walletClients.set(chainConfig.chainId, walletClient)
		}

		return this.walletClients.get(chainConfig.chainId)!
	}
}

// Create a singleton instance of the factory
export const viemClientFactory = new ViemClientFactory()

/**
 * Manages chain clients for different operations
 */
export class ChainClientManager {
	private privateKey: HexString
	private configService: ChainConfigService
	private clientFactory: ViemClientFactory

	constructor(privateKey: HexString) {
		this.privateKey = privateKey
		this.configService = new ChainConfigService()
		this.clientFactory = viemClientFactory
	}

	getPublicClient(chain: string): PublicClient {
		const config = this.configService.getChainConfig(chain)
		return this.clientFactory.getPublicClient(config)
	}

	getWalletClient(chain: string): WalletClient {
		const config = this.configService.getChainConfig(chain)
		return this.clientFactory.getWalletClient(config, this.privateKey)
	}

	getClientsForOrder(order: Order): {
		destClient: PublicClient
		sourceClient: PublicClient
		walletClient: WalletClient
	} {
		return {
			destClient: this.getPublicClient(order.destChain),
			sourceClient: this.getPublicClient(order.sourceChain),
			walletClient: this.getWalletClient(order.destChain),
		}
	}
}
