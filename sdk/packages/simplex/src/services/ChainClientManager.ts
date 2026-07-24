import {
	PublicClient,
	WalletClient,
	createPublicClient,
	createWalletClient,
	fallback,
	http,
	type Chain,
	type Transport,
} from "viem"
import { generatePrivateKey } from "viem/accounts"
import { Order, ChainConfig, getViemChain } from "@hyperbridge/sdk"
import type { Account } from "viem/accounts"
import { FillerConfigService } from "./FillerConfigService"
import { QuorumPublicClient } from "./QuorumPublicClient"
import type { SigningAccount } from "./wallet"
import { createPrivateKeySigningAccount } from "./wallet/accounts/privatekey"

const HTTP_TRANSPORT_OPTS = {
	timeout: 30_000, // 30 seconds
	retryCount: 3,
	retryDelay: 1000,
} as const

/**
 * Builds the viem transport for a chain. If more than one RPC URL is configured,
 * a `fallback` transport is returned so the public / wallet clients can failover
 * to a healthy provider on timeouts or 5xx responses. A single URL uses plain
 * `http`. This is distinct from `QuorumPublicClient`, which runs every provider
 * in parallel and cross-checks results — the regular clients only need
 * availability, not consensus.
 */
function buildTransport(rpcUrls: readonly string[]): Transport {
	if (rpcUrls.length === 1) {
		return http(rpcUrls[0], HTTP_TRANSPORT_OPTS)
	}
	return fallback(rpcUrls.map((url) => http(url, HTTP_TRANSPORT_OPTS)))
}

function walletClientCacheKey(chainId: number, accountAddress: string): string {
	return `${chainId}:${accountAddress.toLowerCase()}`
}

/**
 * Factory for creating and managing Viem clients
 */
class ViemClientFactoryImpl {
	private publicClients: Map<number, PublicClient> = new Map()
	private walletClients: Map<string, WalletClient<Transport, Chain, Account>> = new Map()

	public getPublicClient(chainConfig: ChainConfig, rpcUrls?: readonly string[]): PublicClient {
		if (!this.publicClients.has(chainConfig.chainId)) {
			const chain = getViemChain(chainConfig.chainId) as Chain
			const urls = rpcUrls && rpcUrls.length > 0 ? rpcUrls : [chainConfig.rpcUrl]

			const publicClient = createPublicClient({
				chain,
				transport: buildTransport(urls),
			})

			this.publicClients.set(chainConfig.chainId, publicClient)
		}

		return this.publicClients.get(chainConfig.chainId)!
	}

	public getWalletClient(
		chainConfig: ChainConfig,
		account: Account,
		rpcUrls?: readonly string[],
	): WalletClient<Transport, Chain, Account> {
		const key = walletClientCacheKey(chainConfig.chainId, account.address)
		if (!this.walletClients.has(key)) {
            const chain = getViemChain(chainConfig.chainId) as Chain
			const urls = rpcUrls && rpcUrls.length > 0 ? rpcUrls : [chainConfig.rpcUrl]

			const walletClient = createWalletClient({
				chain,
				account,
				transport: buildTransport(urls),
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
	private quorumClients: Map<number, QuorumPublicClient> = new Map()

	constructor(configService: FillerConfigService, signer?: SigningAccount) {
		this.configService = configService
		this.clientFactory = ViemClientFactory
		this.signer = signer ?? createPrivateKeySigningAccount(generatePrivateKey())
	}

	getPublicClient(chain: string): PublicClient {
		const config = this.configService.getChainConfig(chain)
		const rpcUrls = this.configService.getRpcUrls(chain)
		return this.clientFactory.getPublicClient(config, rpcUrls)
	}

	/**
	 * Quorum client for consensus-critical reads (event scanning, cross-chain
	 * confirmation counting). Built from the operator's endpoints plus the public
	 * RPC registry (`FillerConfigService.getQuorumRpcUrls`) and cached per chain,
	 * so the event monitor and the confirmation waiter share one provider set.
	 */
	getQuorumClient(chain: string): QuorumPublicClient {
		const config = this.configService.getChainConfig(chain)
		let client = this.quorumClients.get(config.chainId)
		if (!client) {
			// getQuorumRpcUrls puts the operator's endpoints first, then the public
			// registry — the operator count tells the quorum client which providers
			// may never be ejected.
			const operatorCount = this.configService.getRpcUrls(chain).length
			client = new QuorumPublicClient(config.chainId, this.configService.getQuorumRpcUrls(chain), operatorCount)
			this.quorumClients.set(config.chainId, client)
		}
		return client
	}

	getWalletClient(chain: string): WalletClient<Transport, Chain, Account> {
		const config = this.configService.getChainConfig(chain)
		const rpcUrls = this.configService.getRpcUrls(chain)
		return this.clientFactory.getWalletClient(config, this.signer.account, rpcUrls)
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
