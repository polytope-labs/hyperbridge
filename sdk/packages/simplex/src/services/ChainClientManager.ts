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
import { parseChainKey } from "@/config/interpolated-curve"
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
 * Factory for creating and managing Viem clients.
 *
 * One instance per `ChainClientManager` — never process-wide. A shared factory
 * would hand the same cached client to two fillers running in one process (the
 * library exposes `Simplex.start` more than once), so one instance's RPC
 * endpoints would silently serve the other's requests. Per-instance caching also
 * makes `invalidate` meaningful: an RPC swap only has to drop this filler's
 * clients.
 */
class ViemClientFactoryImpl {
	private publicClients: Map<number, PublicClient> = new Map()
	private walletClients: Map<string, WalletClient<Transport, Chain, Account>> = new Map()

	/**
	 * Drops every cached client for a chain so the next `get*` call rebuilds it
	 * from the current RPC URLs. Clients are cached by chain id (and account, for
	 * wallet clients) with the transport baked in at construction, so a URL
	 * change is invisible without this.
	 */
	public invalidate(chainId: number): void {
		this.publicClients.delete(chainId)
		for (const key of [...this.walletClients.keys()]) {
			if (key.startsWith(`${chainId}:`)) this.walletClients.delete(key)
		}
	}

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

/**
 * Manages chain clients for different operations
 */
export class ChainClientManager {
	private signer: SigningAccount
	private configService: FillerConfigService
	private clientFactory = new ViemClientFactoryImpl()
	private quorumClients: Map<number, QuorumPublicClient> = new Map()

	constructor(configService: FillerConfigService, signer?: SigningAccount) {
		this.configService = configService
		this.signer = signer ?? createPrivateKeySigningAccount(generatePrivateKey())
	}

	/**
	 * Drops every cached client for a chain. Call after the chain's RPC URLs
	 * change (or when the chain is removed) so the next lookup rebuilds against
	 * the current endpoints — transports are baked into a client at construction.
	 *
	 * Callers that hold a client reference across the invalidation keep talking to
	 * the old endpoints; `EventMonitor` re-reads its quorum client per chain
	 * rebuild for exactly this reason.
	 */
	invalidate(chain: string): void {
		// parseChainKey, not the SDK registry lookup: a chain being removed may
		// already be gone from the resolved set, and invalidation must still work.
		const chainId = parseChainKey(chain)
		if (chainId === null) throw new Error(`Cannot derive chain ID from state machine ID: "${chain}"`)
		this.clientFactory.invalidate(chainId)
		this.quorumClients.delete(chainId)
	}

	getPublicClient(chain: string): PublicClient {
		const config = this.configService.getChainConfig(chain)
		const rpcUrls = this.configService.getRpcUrls(chain)
		return this.clientFactory.getPublicClient(config, rpcUrls)
	}

	/**
	 * Quorum client for consensus-critical reads (event scanning, cross-chain
	 * confirmation counting). Built from the operator's configured endpoints and
	 * cached per chain, so the event monitor and the confirmation waiter share
	 * one provider set.
	 */
	getQuorumClient(chain: string): QuorumPublicClient {
		const config = this.configService.getChainConfig(chain)
		let client = this.quorumClients.get(config.chainId)
		if (!client) {
			client = new QuorumPublicClient(config.chainId, this.configService.getRpcUrls(chain))
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
