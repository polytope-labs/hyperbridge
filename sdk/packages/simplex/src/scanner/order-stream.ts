import { ChainConfigService, type HexString } from "@hyperbridge/sdk"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { fetchChainId, validateRpcUrls } from "@/services/FillerConfigService"
import { ChainScanner } from "./chain-scanner"
import { FanOut } from "./fan-out"
import type {
	OrderStream as OrderStreamContract,
	OrderStreamHandlers,
	ScannedFill,
	ScannedOrder,
	StreamChainConfig,
	Subscription,
} from "./types"

/**
 * One scan loop per chain, feeding every filler you hand it to.
 *
 * ```ts
 * const orders = await OrderStream.create(config.chains)
 * const eu = await Simplex.start({ config: euConfig, orderStream: orders })
 * const apac = await Simplex.start({ config: apacConfig, orderStream: orders })
 * await orders.close()
 * ```
 *
 * You own it: the fillers you pass it to subscribe and unsubscribe, but none of
 * them closes it. A filler started without one builds a private stream from its
 * own config and closes it on `stop()`.
 */
export class OrderStream implements OrderStreamContract {
	private readonly scanners = new Map<number, ChainScanner>()
	private readonly orders = new FanOut<ScannedOrder>("orders")
	private readonly fills = new FanOut<ScannedFill>("fills")
	private readonly errors = new Set<(error: unknown, chainId: number) => void>()
	private readonly logger: Logger
	private closed = false

	private constructor(private readonly loggers: LoggerContext) {
		this.logger = loggers.get("order-stream")
	}

	/**
	 * Resolves each chain and starts scanning it.
	 *
	 * `chains` takes the same shape as the filler config's `chains` array, so it
	 * can be passed straight through. Any entry without a `chainId` has it read
	 * back from its own endpoints, which also proves they answer for one chain
	 * before a filler ever depends on them.
	 *
	 * @throws if an endpoint set is invalid or unreachable, or if two entries
	 *   resolve to the same chain.
	 */
	static async create(
		chains: StreamChainConfig[],
		options: { loggers?: LoggerContext } = {},
	): Promise<OrderStream> {
		const stream = new OrderStream(options.loggers ?? defaultLoggerContext())
		// Sequential: two entries resolving to the same chain must be caught, and the
		// only network call is the chain-id probe for entries that omit it.
		for (const chain of chains) await stream.addChain(chain)
		return stream
	}

	subscribe(handlers: OrderStreamHandlers): Subscription {
		if (this.closed) throw new Error("This OrderStream is closed")

		const orderConsumer = this.orders.add(handlers.onOrder)
		const fillConsumer = this.fills.add(handlers.onFill)
		if (handlers.onError) this.errors.add(handlers.onError)

		return {
			close: () => {
				orderConsumer.close()
				fillConsumer.close()
				if (handlers.onError) this.errors.delete(handlers.onError)
			},
			get dropped() {
				return orderConsumer.dropped + fillConsumer.dropped
			},
		}
	}

	chains(): number[] {
		return [...this.scanners.keys()]
	}

	/** Events dropped across every subscriber, for health reporting. */
	get subscriberCount(): number {
		return this.orders.size
	}

	async addChain(chain: StreamChainConfig): Promise<number> {
		if (this.closed) throw new Error("This OrderStream is closed")

		const rpcUrls = validateRpcUrls(chain.rpcUrls)
		const chainId = chain.chainId ?? (await fetchChainId(rpcUrls[0]))
		if (this.scanners.has(chainId)) {
			throw new Error(`Chain ${chainId} is already in this stream`)
		}

		const name = `EVM-${chainId}`
		const gateway = chain.gateway ?? (new ChainConfigService({}).getIntentGatewayAddress(name) as HexString)

		const scanner = new ChainScanner({ chain: name, chainId, gateway, rpcUrls }, this.loggers)
		scanner.onOrder((event) => this.orders.publish(event))
		scanner.onFill((event) => this.fills.publish(event))
		scanner.onError((error) => {
			for (const handler of this.errors) {
				try {
					handler(error, chainId)
				} catch {
					// One subscriber's error handler must not break the stream for the rest.
				}
			}
		})

		this.scanners.set(chainId, scanner)
		scanner.start()
		this.logger.info({ chainId, gateway }, "Streaming gateway events")
		return chainId
	}

	async removeChain(chainId: number): Promise<void> {
		const scanner = this.scanners.get(chainId)
		if (!scanner) return
		this.scanners.delete(chainId)
		await scanner.stop()
		this.logger.info({ chainId }, "Stopped streaming gateway events")
	}

	async close(): Promise<void> {
		if (this.closed) return
		this.closed = true
		await Promise.all([...this.scanners.values()].map((scanner) => scanner.stop()))
		this.scanners.clear()
		this.errors.clear()
		this.logger.info("Order stream closed")
	}
}
