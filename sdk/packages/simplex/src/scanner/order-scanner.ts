import { ChainConfigService, type HexString } from "@hyperbridge/sdk"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { fetchChainId, MIN_BLOCK_SCAN_INTERVAL_SECONDS, validateRpcUrls } from "@/services/FillerConfigService"
import { ChainScanner } from "./chain-scanner"
import { FanOut } from "./fan-out"
import type {
	OrderScanner as OrderScannerContract,
	OrderScannerHandlers,
	OrderScannerOptions,
	ScannedFill,
	ScannedOrder,
	ScannerChainConfig,
	Subscription,
} from "./types"

/**
 * One scan loop per chain, feeding every filler you hand it to.
 *
 * ```ts
 * const orders = await OrderScanner.create({ chains: config.chains })
 * const eu = await Simplex.start({ config: euConfig, orderScanner: orders })
 * const apac = await Simplex.start({ config: apacConfig, orderScanner: orders })
 * await orders.close()
 * ```
 *
 * You own it: the fillers you pass it to subscribe and unsubscribe, but none of
 * them closes it. A filler started without one builds a private scanner from its
 * own config and closes it on `stop()`.
 */
export class OrderScanner implements OrderScannerContract {
	private readonly scanners = new Map<number, ChainScanner>()
	private readonly orders: FanOut<ScannedOrder>
	private readonly fills: FanOut<ScannedFill>
	private readonly errors = new Set<(error: unknown, chainId: number) => void>()
	private readonly logger: Logger
	private closed = false

	private constructor(
		private readonly loggers: LoggerContext,
		private readonly scanIntervalMs?: number,
	) {
		this.logger = loggers.get("order-scanner")
		// Built here, not as field initialisers: those run before `loggers` is
		// assigned, so the fan-outs would fall back to the process-wide context and
		// a filler with its own sinks would leak these records into the host.
		this.orders = new FanOut<ScannedOrder>("orders", loggers)
		this.fills = new FanOut<ScannedFill>("fills", loggers)
	}

	/**
	 * Resolves each chain and starts scanning it.
	 *
	 * `chains` takes the same shape as the filler config's `chains` array, so it
	 * can be passed straight through. Any entry without a `chainId` has it read
	 * back from its own endpoints, which also proves they answer for one chain
	 * before a filler ever depends on them.
	 *
	 * @throws if an endpoint set is invalid or unreachable, if two entries resolve
	 *   to the same chain, or if `scanIntervalSecs` is below the minimum.
	 */
	static async create(options: OrderScannerOptions): Promise<OrderScanner> {
		const { chains, scanIntervalSecs, loggers } = options
		if (scanIntervalSecs !== undefined) {
			// Same gate the TOML validator applies: a zero or negative interval would
			// spin the loop as fast as the event loop allows and exhaust an RPC budget
			// in minutes, and setInterval would coerce it rather than complain.
			if (!Number.isFinite(scanIntervalSecs) || scanIntervalSecs < MIN_BLOCK_SCAN_INTERVAL_SECONDS) {
				throw new Error(
					`scanIntervalSecs must be a number >= ${MIN_BLOCK_SCAN_INTERVAL_SECONDS} (seconds); got ${scanIntervalSecs}`,
				)
			}
		}

		const scanner = new OrderScanner(
			loggers ?? defaultLoggerContext(),
			scanIntervalSecs === undefined ? undefined : Math.round(scanIntervalSecs * 1000),
		)
		try {
			// Sequential: two entries resolving to the same chain must be caught, and the
			// only network call is the chain-id probe for entries that omit it.
			for (const chain of chains) await scanner.addChain(chain)
		} catch (error) {
			// The caller never gets a handle to a rejected create, so the loops already
			// started here would scan forever with nobody able to stop them.
			await scanner.close().catch(() => {})
			throw error
		}
		return scanner
	}

	subscribe(handlers: OrderScannerHandlers): Subscription {
		if (this.closed) throw new Error("This OrderScanner is closed")

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

	async addChain(chain: ScannerChainConfig): Promise<number> {
		if (this.closed) throw new Error("This OrderScanner is closed")

		const rpcUrls = validateRpcUrls(chain.rpcUrls)
		const chainId = chain.chainId ?? (await fetchChainId(rpcUrls[0]))
		if (this.scanners.has(chainId)) {
			throw new Error(`Chain ${chainId} is already in this scanner`)
		}

		const name = `EVM-${chainId}`
		const gateway = chain.gateway ?? (new ChainConfigService({}).getIntentGatewayAddress(name) as HexString)

		const scanner = new ChainScanner(
			{ chain: name, chainId, gateway, rpcUrls },
			this.loggers,
			this.scanIntervalMs,
		)
		scanner.onOrder((event) => this.orders.publish(event))
		scanner.onFill((event) => this.fills.publish(event))
		scanner.onError((error) => {
			for (const handler of this.errors) {
				try {
					handler(error, chainId)
				} catch {
					// One subscriber's error handler must not break the scanner for the rest.
				}
			}
		})

		this.scanners.set(chainId, scanner)
		scanner.start()
		this.logger.info({ chainId, gateway }, "Scanning gateway events")
		return chainId
	}

	/**
	 * Points a chain's scan loop at different endpoints, keeping its cursor.
	 *
	 * The new endpoints are probed first and must answer for the chain being
	 * edited — pointing Base's loop at a Polygon endpoint would otherwise publish
	 * Polygon orders tagged as Base to every filler reading this scanner. Nothing
	 * is mutated until that check passes.
	 *
	 * @throws if the chain is not in this scanner, or the endpoints are invalid,
	 *   unreachable, or answer for a different chain.
	 */
	async setRpcUrls(chainId: number, rpcUrls: string[]): Promise<void> {
		if (this.closed) throw new Error("This OrderScanner is closed")

		const scanner = this.scanners.get(chainId)
		if (!scanner) throw new Error(`Chain ${chainId} is not in this scanner`)

		const validated = validateRpcUrls(rpcUrls)
		const answered = await fetchChainId(validated[0])
		if (answered !== chainId) {
			throw new Error(`Endpoints for chain ${chainId} answer for chain ${answered}`)
		}

		await scanner.setRpcUrls(validated)
		this.logger.info({ chainId }, "Swapped gateway event endpoints")
	}

	async removeChain(chainId: number): Promise<void> {
		const scanner = this.scanners.get(chainId)
		if (!scanner) return
		this.scanners.delete(chainId)
		await scanner.stop()
		this.logger.info({ chainId }, "Stopped scanning gateway events")
	}

	async close(): Promise<void> {
		if (this.closed) return
		this.closed = true
		await Promise.all([...this.scanners.values()].map((scanner) => scanner.stop()))
		this.scanners.clear()
		this.errors.clear()
		this.logger.info("Order scanner closed")
	}
}
