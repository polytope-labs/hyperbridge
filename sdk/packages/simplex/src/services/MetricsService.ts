import { createServer, type Server } from "node:http"
import { readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import { networkInterfaces } from "node:os"
import { Registry, Counter, Gauge, Histogram, collectDefaultMetrics } from "prom-client"
import type { EventMonitor } from "@/core/event-monitor"
import type { BidStorageService } from "./BidStorageService"
import type { BalanceProvider } from "./BalanceProvider"
import { getLogger } from "./Logger"

function getLocalNetworkIp(): string | null {
	const nets = networkInterfaces()
	for (const iface of Object.values(nets)) {
		for (const net of iface ?? []) {
			if (net.family === "IPv4" && !net.internal) {
				return net.address
			}
		}
	}
	return null
}

export interface MetricsServiceOptions {
	monitor: EventMonitor
	bidStorage: BidStorageService | undefined
	balances: BalanceProvider
	dataDir?: string
}

interface PersistedCounters {
	ordersDetected: number
	ordersFilled: number
	ordersSkipped: number
	orderVolumeUsd: Record<string, number>
	orderProfitUsd: Record<string, number>
}

export class MetricsService {
	private server: Server
	private registry: Registry
	private logger = getLogger("metrics")
	private options: MetricsServiceOptions

	// Counters
	private ordersDetectedTotal: Counter
	private ordersFilledTotal: Counter
	private ordersExecutedTotal: Counter
	private ordersSkippedTotal: Counter
	private bidsSubmittedTotal: Counter
	private orderVolumeUsdTotal: Counter
	private orderProfitUsdTotal: Counter

	// Gauges
	private balanceUsdc: Gauge
	private balanceUsdt: Gauge
	private balanceNative: Gauge
	private balanceExotic: Gauge
	private bidsPending: Gauge
	private bidsSuccessful: Gauge
	private bidsFailed: Gauge
	private bidsRetracted: Gauge
	private hyperbridgeFree: Gauge
	private hyperbridgeReserved: Gauge
	private uptimeSeconds: Gauge

	// Histograms
	private orderProcessingDuration: Histogram
	private orderPhaseDuration: Histogram

	private startTime: number
	private persistPath: string | undefined
	private persistInterval?: NodeJS.Timeout

	constructor(options: MetricsServiceOptions) {
		this.options = options
		this.startTime = Date.now()
		this.registry = new Registry()
		this.persistPath = options.dataDir ? join(options.dataDir, "metrics-state.json") : undefined

		collectDefaultMetrics({ register: this.registry, prefix: "simplex_" })

		// ─── Counters ────────────────────────────────────────────────────────────

		this.ordersDetectedTotal = new Counter({
			name: "simplex_orders_detected_total",
			help: "Total number of orders detected on-chain",
			registers: [this.registry],
		})

		this.ordersFilledTotal = new Counter({
			name: "simplex_orders_filled_total",
			help: "Total number of orders filled",
			registers: [this.registry],
		})

		this.ordersExecutedTotal = new Counter({
			name: "simplex_orders_executed_total",
			help: "Total number of orders executed (includes failures)",
			labelNames: ["success", "strategy"],
			registers: [this.registry],
		})

		this.ordersSkippedTotal = new Counter({
			name: "simplex_orders_skipped_total",
			help: "Total number of orders skipped (not profitable)",
			registers: [this.registry],
		})

		this.bidsSubmittedTotal = new Counter({
			name: "simplex_bids_submitted_total",
			help: "Total bids submitted to Hyperbridge",
			labelNames: ["success"],
			registers: [this.registry],
		})

		this.orderVolumeUsdTotal = new Counter({
			name: "simplex_order_volume_usd_total",
			help: "Cumulative USD volume of filled orders",
			labelNames: ["chain_id"],
			registers: [this.registry],
		})

		this.orderProfitUsdTotal = new Counter({
			name: "simplex_order_profit_usd_total",
			help: "Cumulative USD profit from bid/ask spread on filled orders",
			labelNames: ["chain_id"],
			registers: [this.registry],
		})

		// ─── Gauges ──────────────────────────────────────────────────────────────

		this.balanceUsdc = new Gauge({
			name: "simplex_balance_usdc",
			help: "USDC balance per chain",
			labelNames: ["chain_id"],
			registers: [this.registry],
		})

		this.balanceUsdt = new Gauge({
			name: "simplex_balance_usdt",
			help: "USDT balance per chain",
			labelNames: ["chain_id"],
			registers: [this.registry],
		})

		this.balanceNative = new Gauge({
			name: "simplex_balance_native",
			help: "Native token balance per chain",
			labelNames: ["chain_id", "symbol"],
			registers: [this.registry],
		})

		this.balanceExotic = new Gauge({
			name: "simplex_balance_exotic",
			help: "Non-USD token balance per chain",
			labelNames: ["chain_id", "symbol"],
			registers: [this.registry],
		})

		this.bidsPending = new Gauge({
			name: "simplex_bids_pending",
			help: "Number of bids pending retraction",
			registers: [this.registry],
		})

		this.bidsSuccessful = new Gauge({
			name: "simplex_bids_successful",
			help: "Total successful bids (from SQLite)",
			registers: [this.registry],
		})

		this.bidsFailed = new Gauge({
			name: "simplex_bids_failed",
			help: "Total failed bids (from SQLite)",
			registers: [this.registry],
		})

		this.bidsRetracted = new Gauge({
			name: "simplex_bids_retracted",
			help: "Total retracted bids (from SQLite)",
			registers: [this.registry],
		})

		this.hyperbridgeFree = new Gauge({
			name: "simplex_hyperbridge_balance_free",
			help: "Hyperbridge substrate free balance",
			registers: [this.registry],
		})

		this.hyperbridgeReserved = new Gauge({
			name: "simplex_hyperbridge_balance_reserved",
			help: "Hyperbridge substrate reserved balance",
			registers: [this.registry],
		})

		this.uptimeSeconds = new Gauge({
			name: "simplex_uptime_seconds",
			help: "Process uptime in seconds",
			registers: [this.registry],
		})

		// ─── Histogram ───────────────────────────────────────────────────────────

		this.orderProcessingDuration = new Histogram({
			name: "simplex_order_processing_duration_seconds",
			help: "Time from order detection to execution result",
			labelNames: ["success"],
			buckets: [0.5, 1, 2, 5, 10, 30, 60, 120],
			registers: [this.registry],
		})

		this.orderPhaseDuration = new Histogram({
			name: "simplex_order_phase_duration_seconds",
			help: "Duration of individual order processing phases",
			labelNames: ["phase"],
			buckets: [0.1, 0.5, 1, 2, 5, 10, 30, 60, 120],
			registers: [this.registry],
		})

		this.restoreCounters()

		this.server = createServer(async (req, res) => {
			const path = (req.url ?? "/").split("?")[0]

			if (path === "/metrics") {
				this.uptimeSeconds.set((Date.now() - this.startTime) / 1000)
				this.refreshBidStats()
				this.applyBalanceSnapshot()

				res.writeHead(200, { "Content-Type": this.registry.contentType })
				res.end(await this.registry.metrics())
			} else if (path === "/health") {
				res.writeHead(200, { "Content-Type": "application/json" })
				res.end(JSON.stringify({ status: "ok" }))
			} else {
				res.writeHead(404, { "Content-Type": "text/plain" })
				res.end("Not found")
			}
		})

		this.setupMonitorListeners()
	}

	start(port: number, host = "0.0.0.0"): void {
		this.server.listen(port, host, () => {
			this.logger.info({ bind: `${host}:${port}` }, `Prometheus metrics available at http://localhost:${port}/metrics`)

			if (host === "0.0.0.0") {
				const localIp = getLocalNetworkIp()
				if (localIp) {
					this.logger.info(`Metrics also reachable at http://${localIp}:${port}/metrics`)
				}
			}
		})

		// Persist counters every 30s
		if (this.persistPath) {
			this.persistInterval = setInterval(() => this.persistCounters(), 30_000)
		}
	}

	stop(): void {
		if (this.persistInterval) clearInterval(this.persistInterval)
		this.persistCounters()
		this.server.close()
	}

	private restoreCounters(): void {
		if (!this.persistPath) return
		try {
			const raw = readFileSync(this.persistPath, "utf-8")
			const saved: PersistedCounters = JSON.parse(raw)
			if (saved.ordersDetected > 0) this.ordersDetectedTotal.inc(saved.ordersDetected)
			if (saved.ordersFilled > 0) this.ordersFilledTotal.inc(saved.ordersFilled)
			if (saved.ordersSkipped > 0) this.ordersSkippedTotal.inc(saved.ordersSkipped)
			for (const [chainId, val] of Object.entries(saved.orderVolumeUsd ?? {})) {
				if (val > 0) this.orderVolumeUsdTotal.inc({ chain_id: chainId }, val)
			}
			for (const [chainId, val] of Object.entries(saved.orderProfitUsd ?? {})) {
				if (val > 0) this.orderProfitUsdTotal.inc({ chain_id: chainId }, val)
			}
			this.logger.info({ path: this.persistPath }, "Restored counter state from disk")
		} catch {
			// No saved state or parse error — start fresh
		}
	}

	private async persistCounters(): Promise<void> {
		if (!this.persistPath) return
		try {
			const volumeMetric = await this.orderVolumeUsdTotal.get()
			const profitMetric = await this.orderProfitUsdTotal.get()

			const orderVolumeUsd: Record<string, number> = {}
			for (const v of volumeMetric.values) {
				orderVolumeUsd[v.labels.chain_id as string] = v.value
			}
			const orderProfitUsd: Record<string, number> = {}
			for (const v of profitMetric.values) {
				orderProfitUsd[v.labels.chain_id as string] = v.value
			}

			const state: PersistedCounters = {
				ordersDetected: (await this.ordersDetectedTotal.get()).values[0]?.value ?? 0,
				ordersFilled: (await this.ordersFilledTotal.get()).values[0]?.value ?? 0,
				ordersSkipped: (await this.ordersSkippedTotal.get()).values[0]?.value ?? 0,
				orderVolumeUsd,
				orderProfitUsd,
			}
			writeFileSync(this.persistPath, JSON.stringify(state))
		} catch (err) {
			this.logger.warn({ err }, "Failed to persist counter state")
		}
	}

	// ─── Balances from the shared provider ───────────────────────────────────────

	private applyBalanceSnapshot(): void {
		const snapshot = this.options.balances.getSnapshot()
		for (const row of snapshot.chains) {
			const chainLabel = String(row.chainId)
			if (row.native) {
				this.balanceNative.set({ chain_id: chainLabel, symbol: row.native.symbol }, row.native.amount)
			}
			if (row.usdc !== undefined) this.balanceUsdc.set({ chain_id: chainLabel }, row.usdc)
			if (row.usdt !== undefined) this.balanceUsdt.set({ chain_id: chainLabel }, row.usdt)
			if (row.exotic) {
				this.balanceExotic.set({ chain_id: chainLabel, symbol: row.exotic.symbol }, row.exotic.amount)
			}
		}
		if (snapshot.hyperbridge) {
			this.hyperbridgeFree.set(snapshot.hyperbridge.free)
			this.hyperbridgeReserved.set(snapshot.hyperbridge.reserved)
		}
	}

	// ─── Monitor Listeners ────────────────────────────────────────────────────────

	private orderDetectionTimes = new Map<string, number>()

	private setupMonitorListeners(): void {
		const { monitor } = this.options

		monitor.on("newOrder", ({ order }) => {
			this.ordersDetectedTotal.inc()
			this.orderDetectionTimes.set(order.id, Date.now())
		})

		monitor.on("orderFilled", ({ volumeUsd, profitUsd, chainId }: { volumeUsd?: number; profitUsd?: number; chainId?: number }) => {
			this.ordersFilledTotal.inc()
			const chain = String(chainId ?? "unknown")
			if (volumeUsd != null && volumeUsd > 0) {
				this.orderVolumeUsdTotal.inc({ chain_id: chain }, volumeUsd)
			}
			if (profitUsd != null && profitUsd > 0) {
				this.orderProfitUsdTotal.inc({ chain_id: chain }, profitUsd)
			}
		})

		monitor.on("orderExecuted", ({ orderId, success, strategy, commitment, error }) => {
			this.ordersExecutedTotal.inc({ success: String(success), strategy: strategy ?? "unknown" })

			// Record processing duration
			const detectedAt = this.orderDetectionTimes.get(orderId)
			if (detectedAt) {
				const durationSec = (Date.now() - detectedAt) / 1000
				this.orderProcessingDuration.observe({ success: String(success) }, durationSec)
				this.orderDetectionTimes.delete(orderId)
			}

			if (commitment) {
				this.bidsSubmittedTotal.inc({ success: String(success) })
			}
		})

		monitor.on("orderSkipped", () => {
			this.ordersSkippedTotal.inc()
		})

		monitor.on("orderTiming", ({ phase, durationSec }: { orderId: string; phase: string; durationSec: number }) => {
			this.orderPhaseDuration.observe({ phase }, durationSec)
		})
	}

	// ─── Bid Stats from SQLite ───────────────────────────────────────────────────

	private refreshBidStats(): void {
		if (!this.options.bidStorage) return
		const stats = this.options.bidStorage.getStats()
		this.bidsSuccessful.set(stats.successful)
		this.bidsFailed.set(stats.failed)
		this.bidsRetracted.set(stats.retracted)
		this.bidsPending.set(stats.pendingRetraction)
	}

}
