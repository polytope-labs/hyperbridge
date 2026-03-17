import { IntentsCoprocessor, type HexString, type PriceInput } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"

export interface PriceUpdateConfig {
	/** How often to submit price updates, in seconds. Default: 300 (5 min) */
	intervalSeconds?: number
	/** Token pairs and their price entries to submit */
	pairs: PriceUpdatePairEntry[]
}

export interface PriceUpdatePairEntry {
	/** The pair ID (H256 / bytes32) */
	pairId: HexString
	/** Human-readable label for logging */
	label?: string
	/** Price entries to submit */
	entries: PriceInput[]
}

/**
 * Periodically submits price updates to the intents coprocessor on Hyperbridge.
 */
export class PriceUpdateService {
	private interval?: NodeJS.Timeout
	private logger = getLogger("price-updates")

	constructor(
		private hyperbridge: Promise<IntentsCoprocessor>,
		private config: PriceUpdateConfig,
	) {}

	/**
	 * Start the periodic price update loop.
	 */
	start(): void {
		const intervalMs = (this.config.intervalSeconds ?? 300) * 1000

		// Run an initial submission after a short delay
		setTimeout(() => {
			this.submitAll().catch((err) => {
				this.logger.error({ err }, "Error in initial price submission")
			})
		}, 5_000)

		this.interval = setInterval(() => {
			this.submitAll().catch((err) => {
				this.logger.error({ err }, "Error in periodic price submission")
			})
		}, intervalMs)

		this.logger.info(
			{ intervalSeconds: this.config.intervalSeconds ?? 300, pairCount: this.config.pairs.length },
			"Price update service started",
		)
	}

	/**
	 * Stop the periodic price update loop.
	 */
	stop(): void {
		if (this.interval) {
			clearInterval(this.interval)
			this.interval = undefined
			this.logger.info("Price update service stopped")
		}
	}

	/**
	 * Submit prices for all configured pairs.
	 */
	async submitAll(): Promise<void> {
		const coprocessor = await this.hyperbridge

		for (const pair of this.config.pairs) {
			try {
				const result = await coprocessor.submitPairPrice(pair.pairId, pair.entries)
				if (result.success) {
					this.logger.info(
						{ pairId: pair.pairId, label: pair.label, blockHash: result.blockHash },
						"Price submitted successfully",
					)
				} else {
					this.logger.error(
						{ pairId: pair.pairId, label: pair.label, error: result.error },
						"Failed to submit price",
					)
				}
			} catch (err) {
				this.logger.error({ pairId: pair.pairId, label: pair.label, err }, "Error submitting price")
			}
		}
	}
}
