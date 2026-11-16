import {
	HttpRequestError,
	type PublicClient,
	type WalletClient,
	type WaitForTransactionReceiptParameters,
	createPublicClient,
	createTransport,
	createWalletClient,
	fallback,
	http,
	shouldThrow,
	toFunctionSelector,
	type Chain,
	type EIP1193RequestFn,
	type Transport,
} from "viem"
import { generatePrivateKey } from "viem/accounts"
import { getTransactionCount, waitForTransactionReceipt } from "viem/actions"
import { type Order, type ChainConfig, getViemChain } from "@hyperbridge/sdk"
import type { Account, LocalAccount } from "viem/accounts"
import type { FillerConfigService } from "./FillerConfigService"
import { parseChainKey } from "@/config/interpolated-curve"
import type { LoggerContext } from "./Logger"
import { CONFIRMATION_BUDGET, QuorumPublicClient } from "./QuorumPublicClient"
import type { Signer } from "./wallet"
import { privateKeySigner } from "./wallet/accounts/privatekey"
import { accountFor } from "./wallet/account"

const HTTP_TRANSPORT_OPTS = {
	timeout: 30_000, // 30 seconds
	retryCount: 3,
	retryDelay: 1000,
} as const

/**
 * Builds the wallet client's transport. If more than one RPC URL is configured,
 * a `fallback` transport is returned so the client fails over to a healthy
 * provider on timeouts or 5xx responses. A single URL uses plain `http`.
 *
 * The order is kept on purpose: a transaction is broadcast to one endpoint, and
 * the nonce it is built on has to come from the node it is sent to.
 */
function buildTransport(rpcUrls: readonly string[]): Transport {
	if (rpcUrls.length === 1) {
		return http(rpcUrls[0], HTTP_TRANSPORT_OPTS)
	}
	return fallback(rpcUrls.map((url) => http(url, HTTP_TRANSPORT_OPTS)))
}

/** How long the race waits for an endpoint that has something, once another has said there is nothing. */
export const EMPTY_ANSWER_GRACE_MS = 1_000

/**
 * Whether an answer says there is nothing: no receipt, transaction or block
 * (`null`), no logs (`[]`) or no code (`0x`). A node that is a block behind
 * gives the same answer for something that exists.
 */
function isEmptyAnswer(method: string, result: unknown): boolean {
	if (result === null) return true
	if (Array.isArray(result)) return result.length === 0
	return method === "eth_getCode" && result === "0x"
}

/** The nonces a signature is built on: `EntryPoint.getNonce` for a UserOp, ERC-2612 `nonces` for a permit. */
const NONCE_SELECTORS = [toFunctionSelector("getNonce(address,uint192)"), toFunctionSelector("nonces(address)")]

/**
 * Whether a read goes to the endpoints in list order instead of to whichever answers first.
 *
 * These are the reads a node that is behind answers wrongly without saying so. Its nonce is the one
 * just used, whether the account's, the EntryPoint's or a permit's, and that is never an empty
 * answer the race would wait on. Its logs are an empty list for a range that holds some, and a node
 * that keeps few logs answers the same way at once.
 *
 * A nonce read is looked for anywhere in the calldata, not only at its start, so one batched into
 * a multicall still goes in order. Another call that happens to hold the same four bytes goes in
 * order too, which costs it nothing but the race.
 */
function readsInOrder(method: string, params: unknown): boolean {
	if (method === "eth_getTransactionCount" || method === "eth_getLogs") return true
	if (method !== "eth_call") return false
	const data = (params as [{ data?: unknown }?] | undefined)?.[0]?.data
	return typeof data === "string" && NONCE_SELECTORS.some((selector) => data.includes(selector.slice(2)))
}

/** RPC error codes viem retries: unknown, limit exceeded and internal. */
const RETRIED_RPC_CODES = [-1, -32005, -32603]
/** HTTP statuses viem retries. */
const RETRIED_HTTP_STATUSES = [403, 408, 413, 429, 500, 502, 503, 504]

/** Whether viem retries a request that failed with `error`. Mirrors its `shouldRetry`, which is not exported. */
function isRetried(error: unknown): boolean {
	const code = (error as { code?: unknown } | null)?.code
	if (typeof code === "number") return RETRIED_RPC_CODES.includes(code)
	if (error instanceof HttpRequestError && error.status) return RETRIED_HTTP_STATUSES.includes(error.status)
	return true
}

/**
 * Settles with the first endpoint to answer.
 *
 * A revert is an answer, by the rule `fallback` uses to stop failing over. An
 * endpoint that is unreachable, times out or refuses the call is not, and the
 * race rejects only when all of them fail. It then rejects with a failure viem
 * retries, when there is one, so one endpoint's refusal does not end the read.
 * The read is then retried even though the refusing endpoint will refuse again.
 *
 * An answer `isEmpty` calls empty is taken only when no endpoint has more: once
 * all of them have settled, or `EMPTY_ANSWER_GRACE_MS` after the first empty one.
 */
export function firstResponse<T>(
	requests: readonly Promise<T>[],
	isEmpty: (value: T) => boolean = () => false,
): Promise<T> {
	return new Promise((resolve, reject) => {
		if (requests.length === 0) return reject(new Error("No endpoint to ask"))
		let pending = requests.length
		let settled = false
		let empty: { value: T } | undefined
		let failure: unknown
		let grace: NodeJS.Timeout | undefined
		const settle = (finish: () => void) => {
			settled = true
			clearTimeout(grace)
			finish()
		}
		// Nothing better came: the empty answer if there was one, else the failure.
		const giveUp = () => settle(() => (empty ? resolve(empty.value) : reject(failure)))
		for (const request of requests) {
			request.then(
				(value) => {
					if (settled) return
					if (!isEmpty(value)) return settle(() => resolve(value))
					empty ??= { value }
					grace ??= setTimeout(giveUp, EMPTY_ANSWER_GRACE_MS)
					if (--pending === 0) giveUp()
				},
				(error) => {
					if (settled) return
					if (error instanceof Error && shouldThrow(error)) return settle(() => reject(error))
					if (failure === undefined || (!isRetried(failure) && isRetried(error))) failure = error
					if (--pending === 0) giveUp()
				},
			)
		}
	})
}

/**
 * Builds the public client's transport: a read goes to all of the chain's
 * endpoints at once and the first answer is used, so an endpoint that is slow
 * or down does not hold up a read another endpoint can answer. `fallback` tries
 * them in order instead, and each one that is down holds every read up for its
 * retries.
 *
 * Nonce and log reads are the exception ({@link readsInOrder}). They go to the
 * endpoints in list order, the order the wallet client sends in, and move on
 * only when one fails.
 *
 * Like `fallback`, this only needs availability. `QuorumPublicClient` is the
 * one that cross-checks answers.
 */
export function raceTransport(rpcUrls: readonly string[]): Transport {
	if (rpcUrls.length === 1) {
		return http(rpcUrls[0], HTTP_TRANSPORT_OPTS)
	}
	// The other endpoints are the retry, so none retries on its own.
	const single = rpcUrls.map((url) => http(url, { ...HTTP_TRANSPORT_OPTS, retryCount: 0 }))
	return ({ chain, timeout }) => {
		const endpoints = single.map((endpoint) => endpoint({ chain, retryCount: 0, timeout }))
		// Retried by this transport as a whole, like the race.
		const inOrder = fallback(single, { retryCount: 0 })({ chain, retryCount: 0, timeout })
		return createTransport({
			key: "race",
			name: "Race",
			type: "race",
			request: (({ method, params }) =>
				readsInOrder(method, params)
					? inOrder.request({ method, params })
					: firstResponse(
							endpoints.map((endpoint) => endpoint.request({ method, params })),
							(result) => isEmptyAnswer(method, result),
						)) as EIP1193RequestFn,
		})
	}
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
	private endpointClients: Map<number, PublicClient[]> = new Map()
	private walletClients: Map<string, WalletClient<Transport, Chain, Account>> = new Map()

	/**
	 * Drops every cached client for a chain so the next `get*` call rebuilds it
	 * from the current RPC URLs. Clients are cached by chain id (and account, for
	 * wallet clients) with the transport baked in at construction, so a URL
	 * change is invisible without this.
	 */
	public invalidate(chainId: number): void {
		this.publicClients.delete(chainId)
		this.endpointClients.delete(chainId)
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
				transport: raceTransport(urls),
			}).extend((client) => ({
				/**
				 * Without viem's replacement check. It pairs a block from one endpoint
				 * with a receipt from another, and rejects the wait for a mined
				 * transaction when the second is behind. Nothing here replaces a
				 * transaction or acts on a replacement.
				 */
				waitForTransactionReceipt: (args: WaitForTransactionReceiptParameters<Chain>) =>
					waitForTransactionReceipt(client, { ...args, checkReplacement: args.checkReplacement ?? false }),
			}))

			this.publicClients.set(chainConfig.chainId, publicClient)
		}

		return this.publicClients.get(chainConfig.chainId)!
	}

	public getEndpointClients(chainConfig: ChainConfig, rpcUrls?: readonly string[]): PublicClient[] {
		let clients = this.endpointClients.get(chainConfig.chainId)
		if (!clients) {
			const chain = getViemChain(chainConfig.chainId) as Chain
			const urls = rpcUrls && rpcUrls.length > 0 ? rpcUrls : [chainConfig.rpcUrl]
			clients = urls.map(
				(url) => createPublicClient({ chain, transport: http(url, HTTP_TRANSPORT_OPTS) }) as PublicClient,
			)
			this.endpointClients.set(chainConfig.chainId, clients)
		}
		return clients
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
	private signer: Signer
	/**
	 * The viem account every wallet client is built on, derived from the signer
	 * once. `Signer` is viem-free by design, so this is where the two meet.
	 */
	private account: LocalAccount
	private configService: FillerConfigService
	private clientFactory = new ViemClientFactoryImpl()
	private quorumClients: Map<number, QuorumPublicClient> = new Map()

	/** This filler's logging destination, relayed from the config service. */
	get loggers(): LoggerContext {
		return this.configService.loggers
	}

	constructor(configService: FillerConfigService, signer?: Signer) {
		this.configService = configService
		this.signer = signer ?? privateKeySigner(generatePrivateKey())
		this.account = accountFor(this.signer)
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
	 * One client per endpoint, in list order. They are for a sequence of reads that must describe
	 * one node's view of the chain, such as a head and the logs up to it. The caller runs the whole
	 * sequence on one client and moves to the next only when it fails.
	 *
	 * {@link getPublicClient} can take each read from a different node. So can a client that fails
	 * over per request: the head from one endpoint and, when that one refuses the log range, the
	 * logs from the next.
	 */
	getEndpointClients(chain: string): PublicClient[] {
		const config = this.configService.getChainConfig(chain)
		const rpcUrls = this.configService.getRpcUrls(chain)
		return this.clientFactory.getEndpointClients(config, rpcUrls)
	}

	/**
	 * Quorum client for consensus-critical reads (cross-chain confirmation
	 * counting; the scanner builds its own instance over its own chain list).
	 * Built from the operator's configured endpoints and cached per chain. The
	 * instances are separate but their rate-limit bench is shared state keyed by
	 * endpoint URL, so a throttle either observes benches the endpoint for both.
	 */
	getQuorumClient(chain: string): QuorumPublicClient {
		const config = this.configService.getChainConfig(chain)
		let client = this.quorumClients.get(config.chainId)
		if (!client) {
			// Confirmation polling, not scanning: two sequential requests per
			// endpoint, and the answer gates paying out.
			client = new QuorumPublicClient(
				config.chainId,
				this.configService.getRpcUrls(chain),
				this.loggers,
				CONFIRMATION_BUDGET,
			)
			this.quorumClients.set(config.chainId, client)
		}
		return client
	}

	getWalletClient(chain: string): WalletClient<Transport, Chain, Account> {
		const config = this.configService.getChainConfig(chain)
		const rpcUrls = this.configService.getRpcUrls(chain)
		return this.clientFactory.getWalletClient(config, this.account, rpcUrls)
	}

	/**
	 * The nonce the wallet's next transaction takes, read through the transport that sends it.
	 *
	 * That transport retries an endpoint before it moves to the next. {@link getPublicClient} moves
	 * on at an endpoint's first failure, so after a brief one it would take the nonce from the
	 * second node while the transaction still goes to the first.
	 */
	getPendingNonce(chain: string): Promise<number> {
		return getTransactionCount(this.getWalletClient(chain), { address: this.account.address, blockTag: "pending" })
	}

	getAccount(): Account {
		return this.account
	}

	getSigner(): Signer {
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
