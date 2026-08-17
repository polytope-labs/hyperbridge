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
 * const solverA = await Simplex.start({ config: configA, orderScanner: orders })
 * const solverB = await Simplex.start({ config: configB, orderScanner: orders })
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
	// Boxed per subscription, not a Set of raw functions: two subscribers passing
	// the same handler reference — the common case when one module wires several
	// fillers — would otherwise share a single Set entry, and the first to close
	// would silently deafen the rest.
	private readonly errors = new Set<{ handler: (error: unknown, chainId: number) => void }>()
	private readonly logger: Logger
	private closed = false
	private closing?: Promise<void>
	/** Serialises edits, so one suspended on an RPC probe cannot race close(). */
	private edits: Promise<void> = Promise.resolve()

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
		const errorEntry = handlers.onError ? { handler: handlers.onError } : undefined
		if (errorEntry) this.errors.add(errorEntry)

		return {
			close: () => {
				orderConsumer.close()
				fillConsumer.close()
				if (errorEntry) this.errors.delete(errorEntry)
			},
			get dropped() {
				return orderConsumer.dropped + fillConsumer.dropped
			},
		}
	}

	chains(): number[] {
		return [...this.scanners.keys()]
	}

	/** Live subscribers, for health reporting. */
	get subscriberCount(): number {
		return this.orders.size
	}

	async addChain(chain: ScannerChainConfig): Promise<number> {
		// Serialised: resolving a chain id is a network probe, and an add that
		// suspended there would otherwise resume after a close() that had already
		// seen an empty map — installing a scan loop nothing can reap, on an
		// interval that keeps the host's event loop alive forever.
		const run = this.edits.then(() => this.addChainNow(chain))
		this.edits = run.then(
			() => undefined,
			() => undefined,
		)
		return run
	}

	private async addChainNow(chain: ScannerChainConfig): Promise<number> {
		if (this.closed) throw new Error("This OrderScanner is closed")

		const rpcUrls = validateRpcUrls(chain.rpcUrls)
		const chainId = chain.chainId ?? (await fetchChainId(rpcUrls[0]))
		// Re-checked after the probe: close() may have run while it was in flight.
		if (this.closed) throw new Error("This OrderScanner is closed")
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
			for (const entry of this.errors) {
				try {
					entry.handler(error, chainId)
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
		// Same queue as addChain, for the same reason: this suspends on a network
		// probe, and resuming after close() would swap clients on a stopped loop.
		const run = this.edits.then(() => this.setRpcUrlsNow(chainId, rpcUrls))
		this.edits = run.then(
			() => undefined,
			() => undefined,
		)
		return run
	}

	private async setRpcUrlsNow(chainId: number, rpcUrls: string[]): Promise<void> {
		if (this.closed) throw new Error("This OrderScanner is closed")

		const scanner = this.scanners.get(chainId)
		if (!scanner) throw new Error(`Chain ${chainId} is not in this scanner`)

		const validated = validateRpcUrls(rpcUrls)
		const answered = await fetchChainId(validated[0])
		// Re-checked after the probe: close() may have run while it was in flight.
		if (this.closed) throw new Error("This OrderScanner is closed")
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
		// One teardown, shared: a second close() awaits the first rather than
		// resolving while loops are still draining.
		if (this.closed) return this.closing
		this.closed = true
		this.closing = this.closeNow()
		return this.closing
	}

	private async closeNow(): Promise<void> {
		// Let an addChain that is mid-probe finish and fail its own closed check,
		// so it cannot install a loop behind us.
		await this.edits
		await Promise.all([...this.scanners.values()].map((scanner) => scanner.stop()))
		this.scanners.clear()
		this.errors.clear()
		this.logger.info("Order scanner closed")
	}
}
