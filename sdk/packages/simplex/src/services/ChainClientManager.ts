import {
	PublicClient,
	WalletClient,
	createPublicClient,
	createWalletClient,
	http,
	type Chain,
	type Transport,
} from "viem"
import { generatePrivateKey } from "viem/accounts"
import { Order, ChainConfig, getViemChain } from "@hyperbridge/sdk"
import type { Account } from "viem/accounts"
import { FillerConfigService } from "./FillerConfigService"
import type { SigningAccount } from "./wallet"
import { createSimplexSigner, SignerType } from "./wallet"

function walletClientCacheKey(chainId: number, accountAddress: string): string {
	return `${chainId}:${accountAddress.toLowerCase()}`
}

/**
 * Factory for creating and managing Viem clients
 */
class ViemClientFactoryImpl {
	private publicClients: Map<number, PublicClient> = new Map()
	private walletClients: Map<string, WalletClient<Transport, Chain, Account>> = new Map()

	public getPublicClient(chainConfig: ChainConfig): PublicClient {
		if (!this.publicClients.has(chainConfig.chainId)) {
			const chain = getViemChain(chainConfig.chainId) as Chain

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

	public getWalletClient(chainConfig: ChainConfig, account: Account): WalletClient<Transport, Chain, Account> {
		const key = walletClientCacheKey(chainConfig.chainId, account.address)
		if (!this.walletClients.has(key)) {
			const chain = getViemChain(chainConfig.chainId) as Chain

			const walletClient = createWalletClient({
				chain,
				account,
				transport: http(chainConfig.rpcUrl, {
					timeout: 30000,
					retryCount: 3,
					retryDelay: 1000,
				}),
			})

			this.walletClients.set(key, walletClient)
		}

		return this.walletClients.get(key)!
	}
}

// Create a singleton instance of the factory
export const ViemClientFactory = new ViemClientFactoryImpl()

/**
 * Manages chain clients for different operations
 */
export class ChainClientManager {
	private signer: SigningAccount
	private configService: FillerConfigService
	private clientFactory: ViemClientFactoryImpl

	constructor(configService: FillerConfigService, signer?: SigningAccount) {
		this.configService = configService
		this.clientFactory = ViemClientFactory
		this.signer =
			signer ??
			createSimplexSigner({
				type: SignerType.PrivateKey,
				key: generatePrivateKey(),
			})
	}

	getPublicClient(chain: string): PublicClient {
		const config = this.configService.getChainConfig(chain)
		return this.clientFactory.getPublicClient(config)
	}

	getWalletClient(chain: string): WalletClient<Transport, Chain, Account> {
		const config = this.configService.getChainConfig(chain)
		return this.clientFactory.getWalletClient(config, this.signer.account)
	}

	getAccount(): Account {
		return this.signer.account
	}

	getSigner(): SigningAccount {
		return this.signer
	}

	getClientsForOrder(order: Order): {
		destClient: PublicClient
		sourceClient: PublicClient
		walletClient: WalletClient<Transport, Chain, Account>
	} {
		return {
			destClient: this.getPublicClient(order.destination),
			sourceClient: this.getPublicClient(order.source),
			walletClient: this.getWalletClient(order.destination),
		}
	}
}
