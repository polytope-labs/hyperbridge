import type { ApiPromise } from "@polkadot/api"
import { IntentsCoprocessor, type PhantomOrderEvent } from "@hyperbridge/sdk"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { FanOut } from "./fan-out"
import type { HyperbridgeScanner as HyperbridgeScannerContract, HyperbridgeScannerHandlers, Subscription } from "./types"

/** How long to wait for the node before giving up, rather than retrying forever. */
const CONNECT_TIMEOUT_MS = 30_000

/**
 * One phantom-order poll per Hyperbridge endpoint, feeding any number of fillers.
 *
 * This is the heavier of the two shared scanners. Every filler used to run
 * `pollPhantomOrders` on its own connection, reading *every* Hyperbridge block
 * (block hash, then the events at that block) every 6 seconds — so a fleet
 * multiplied a fixed per-block cost by the instance count against a single node.
 * Phantom order registration is chain-global, so that work is identical for
 * everyone and collapses to one loop.
 *
 * Only reads live here. Bids are signed with an instance's own substrate key and
 * stay on that instance's `IntentsCoprocessor`.
 */
export class HyperbridgeScanner implements HyperbridgeScannerContract {
	private readonly phantom: FanOut<PhantomOrderEvent>
	// Boxed per subscription, not a Set of raw functions: two subscribers passing
	// the same handler reference would share one entry, and the first to close
	// would silently deafen the rest. Matches OrderScanner.
	private readonly errors = new Set<{ handler: (error: unknown) => void }>()
	private readonly logger: Logger

	private connection?: ApiPromise
	private coprocessor?: IntentsCoprocessor
	private stopPolling?: () => void
	private starting?: Promise<void>
	private stopped = false
	/** The connect failure start() swallowed, kept so create() can surface it. */
	private startError?: unknown

	private constructor(
		private readonly wsUrl: string,
		loggers: LoggerContext,
	) {
		this.logger = loggers.get("hyperbridge-scanner")
		// Built here, not as a field initialiser: those run before `loggers` is bound,
		// so the fan-out would fall back to the process-wide context.
		this.phantom = new FanOut<PhantomOrderEvent>("phantom", loggers)
	}

	/**
	 * Connects and begins polling.
	 *
	 * ```ts
	 * const hyperbridge = await HyperbridgeScanner.create(config.simplex.hyperbridgeWsUrl)
	 * const filler = await Simplex.start({ config, hyperbridgeScanner: hyperbridge })
	 * ```
	 *
	 * @throws if the endpoint cannot be reached.
	 */
	static async create(wsUrl: string, options: { loggers?: LoggerContext } = {}): Promise<HyperbridgeScanner> {
		const scanner = new HyperbridgeScanner(wsUrl, options.loggers ?? defaultLoggerContext())
		await scanner.start()
		if (!scanner.connectionOpen) {
			// The cause carries the real failure — timeout, DNS, TLS — which the
			// generic message alone sent operators off to guess at. Assigned, not
			// passed as an option: the compile target predates ErrorOptions.
			const error = new Error(`Could not connect to Hyperbridge at ${wsUrl}`)
			;(error as Error & { cause?: unknown }).cause = scanner.startError
			throw error
		}
		return scanner
	}

	private get connectionOpen(): boolean {
		return Boolean(this.connection)
	}

	subscribe(handlers: HyperbridgeScannerHandlers): Subscription {
		const consumer = this.phantom.add(handlers.onPhantomOrder)
		const errorEntry = handlers.onError ? { handler: handlers.onError } : undefined
		if (errorEntry) this.errors.add(errorEntry)

		return {
			close: () => {
				consumer.close()
				if (errorEntry) this.errors.delete(errorEntry)
			},
			get dropped() {
				return consumer.dropped
			},
		}
	}

	/**
	 * Connects and begins polling. Idempotent, and safe to race: concurrent
	 * subscribers await the same in-flight connect rather than opening a second
	 * socket, which is the whole point of sharing.
	 */
	private async start(): Promise<void> {
		if (this.stopPolling || this.stopped) return
		if (this.starting) return this.starting

		this.starting = (async () => {
			try {
				// The ApiPromise is built here rather than via IntentsCoprocessor.connect so the
				// socket is ours to hand out: BalanceProvider otherwise opens a second connection
				// to the same node per instance purely to read balances. `fromApi` marks the
				// coprocessor as not owning the connection, so only `stop()` closes it.
				const { ApiPromise, WsProvider } = await import("@polkadot/api")
				const provider = new WsProvider(this.wsUrl)
				// `ApiPromise.create` retries a dead endpoint forever rather than
				// rejecting, so without this race `Simplex.start` never settles on a
				// typo'd or unreachable node — no error, no timeout, just a hang.
				const connection = await Promise.race([
					ApiPromise.create({
						provider,
						typesBundle: { spec: { nexus: { hasher: keccakAsU8a }, gargantua: { hasher: keccakAsU8a } } },
					}),
					new Promise<never>((_, reject) =>
						setTimeout(
							() => reject(new Error(`Timed out connecting to Hyperbridge at ${this.wsUrl}`)),
							CONNECT_TIMEOUT_MS,
						).unref?.(),
					),
				]).catch(async (error) => {
					// The provider keeps its reconnect loop running after the race is lost.
					await provider.disconnect().catch(() => {})
					throw error
				})

				// Adopted only after the close check, so a `close()` that landed while we
				// were connecting cannot leave this socket owned by nobody.
				if (this.stopped) {
					await connection.disconnect().catch(() => {})
					return
				}
				this.connection = connection
				// No signing key: nothing on this connection signs.
				this.coprocessor = IntentsCoprocessor.fromApi(this.connection)

				this.stopPolling = this.coprocessor.pollPhantomOrders(
					(order) => this.phantom.publish(order),
					{
						onError: (err) => {
							this.logger.warn({ err }, "Phantom order poll failed, will retry")
							for (const entry of this.errors) {
								try {
									entry.handler(err)
								} catch {
									// One consumer's handler must not break polling for the rest.
								}
							}
						},
					},
				)
				this.logger.info({ wsUrl: this.wsUrl }, "Shared phantom order polling active")
			} catch (err) {
				this.logger.error({ wsUrl: this.wsUrl, err }, "Failed to start shared phantom order polling")
				this.startError = err
				// Clear so a later subscriber retries rather than inheriting a dead scanner.
				this.starting = undefined
			}
		})()

		return this.starting
	}

	/**
	 * The shared read-only connection, for consumers that only read Hyperbridge —
	 * balances, chain state. Saves a second socket per filler.
	 */
	async api(): Promise<ApiPromise | undefined> {
		await this.start()
		return this.connection
	}

	async close(): Promise<void> {
		this.stopped = true
		this.stopPolling?.()
		this.stopPolling = undefined
		try {
			await this.connection?.disconnect()
		} catch (err) {
			this.logger.warn({ wsUrl: this.wsUrl, err }, "Failed to disconnect Hyperbridge scanner cleanly")
		}
		this.coprocessor = undefined
		this.connection = undefined
		this.logger.info({ wsUrl: this.wsUrl }, "Shared phantom order polling stopped")
	}
}
