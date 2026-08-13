import type { ApiPromise } from "@polkadot/api"
import { IntentsCoprocessor, type PhantomOrderEvent } from "@hyperbridge/sdk"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { FanOut } from "./fan-out"
import type { HyperbridgeScanner as HyperbridgeScannerContract, HyperbridgeScannerHandlers, Subscription } from "./types"

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
	private readonly phantom = new FanOut<PhantomOrderEvent>("phantom")
	private readonly errors = new Set<(error: unknown) => void>()
	private readonly logger: Logger

	private connection?: ApiPromise
	private coprocessor?: IntentsCoprocessor
	private stopPolling?: () => void
	private starting?: Promise<void>
	private stopped = false

	private constructor(
		private readonly wsUrl: string,
		loggers: LoggerContext,
	) {
		this.logger = loggers.get("hyperbridge-scanner")
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
			throw new Error(`Could not connect to Hyperbridge at ${wsUrl}`)
		}
		return scanner
	}

	private get connectionOpen(): boolean {
		return Boolean(this.connection)
	}

	subscribe(handlers: HyperbridgeScannerHandlers): Subscription {
		const consumer = this.phantom.add(handlers.onPhantomOrder)
		if (handlers.onError) this.errors.add(handlers.onError)

		return {
			close: () => {
				consumer.close()
				if (handlers.onError) this.errors.delete(handlers.onError)
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
				this.connection = await ApiPromise.create({
					provider: new WsProvider(this.wsUrl),
					typesBundle: { spec: { nexus: { hasher: keccakAsU8a }, gargantua: { hasher: keccakAsU8a } } },
				})
				// No signing key: nothing on this connection signs.
				this.coprocessor = IntentsCoprocessor.fromApi(this.connection)
				if (this.stopped) return

				this.stopPolling = this.coprocessor.pollPhantomOrders(
					(order) => this.phantom.publish(order),
					{
						onError: (err) => {
							this.logger.warn({ err }, "Phantom order poll failed, will retry")
							for (const onError of this.errors) {
								try {
									onError(err)
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
