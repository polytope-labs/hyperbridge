import { defaultLoggerContext, type LoggerContext } from "@/services/Logger"
import { ChainScanner } from "./chain-scanner"
import { HyperbridgeScanner } from "./hyperbridge-scanner"
import { RefCounted } from "./fan-out"
import { scanKey, type HyperbridgeHandlers, type HyperbridgeSource, type OrderSource, type OrderSourceHandlers, type ScanTarget, type Subscription } from "./types"

/**
 * Process-local sharing.
 *
 * Two fillers in one process that watch the same chain through the same
 * endpoints get the same scan loop; the loop starts with the first subscriber
 * and stops with the last. This is the default because it is the topology the
 * `Simplex` class made possible — several fillers in one process — and it needs
 * no deployment of anything.
 *
 * Fillers spread across processes or hosts need a source that speaks over a
 * transport. Implement {@link OrderSource} against these same contracts and pass
 * it as `SimplexOptions.orderSource`; nothing downstream knows the difference.
 */
const chainScanners = new RefCounted<ChainScanner>("chain")
const hyperbridgeScanners = new RefCounted<HyperbridgeScanner>("hyperbridge")

/** Shared EVM gateway-event source, one scan loop per (chain, gateway, endpoints). */
export class SharedOrderSource implements OrderSource {
	constructor(private readonly loggers: LoggerContext = defaultLoggerContext()) {}

	subscribe(target: ScanTarget, handlers: OrderSourceHandlers): Subscription {
		const key = scanKey(target)
		const { value: scanner, release } = chainScanners.acquire(key, () => new ChainScanner(target, this.loggers))
		const subscription = scanner.subscribe(handlers)

		let closed = false
		return {
			close: () => {
				if (closed) return
				closed = true
				subscription.close()
				release()
			},
			get dropped() {
				return subscription.dropped
			},
		}
	}

	activeChains(): number[] {
		return [...new Set(chainScanners.keys().map((key) => Number(key.split(":")[0])))]
	}
}

/** Shared Hyperbridge source, one phantom-order poll per endpoint. */
export class SharedHyperbridgeSource implements HyperbridgeSource {
	constructor(private readonly loggers: LoggerContext = defaultLoggerContext()) {}

	subscribe(wsUrl: string, handlers: HyperbridgeHandlers): Subscription {
		const { value: scanner, release } = hyperbridgeScanners.acquire(
			wsUrl,
			() => new HyperbridgeScanner(wsUrl, this.loggers),
		)
		const subscription = scanner.subscribe(handlers)

		let closed = false
		return {
			close: () => {
				if (closed) return
				closed = true
				subscription.close()
				release()
			},
			get dropped() {
				return subscription.dropped
			},
		}
	}
}

/** Live scan loops, for tests and health reporting. */
export const sharedScanners = {
	chains: () => chainScanners.keys(),
	hyperbridge: () => hyperbridgeScanners.keys(),
	chainScanner: (target: ScanTarget) => chainScanners.peek(scanKey(target)),
}
