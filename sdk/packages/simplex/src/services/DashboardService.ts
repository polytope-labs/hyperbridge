import { createServer, type IncomingMessage, type ServerResponse, type Server } from "node:http"
import { networkInterfaces } from "node:os"
import { formatUnits } from "viem"
import type { EventMonitor } from "@/core/event-monitor"
import type { BidStorageService } from "./BidStorageService"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { getDashboardHtml, type DashboardConfig } from "@/dashboard/html"
import { getLogger } from "./Logger"
import { ERC20_ABI } from "@/config/abis/ERC20"

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

const CHAIN_NATIVE_SYMBOLS: Record<number, string> = {
	1: "ETH",
	56: "BNB",
	137: "MATIC",
	42161: "ETH",
	8453: "ETH",
	10: "ETH",
	43114: "AVAX",
	250: "FTM",
	130: "ETH",
	100: "xDAI",
	97: "BNB",
	11155111: "ETH",
}

interface ActivityEvent {
	type: string
	timestamp: number
	[key: string]: unknown
}

interface ExoticBalance {
	symbol: string
	balance: string
}

interface BalanceEntry {
	usdc: string | null
	usdt: string | null
	native: string | null
	nativeSymbol: string
	exotics: ExoticBalance[]
}

interface HyperbridgeBalance {
	address: string
	free: string
	reserved: string
	decimals: number
	symbol: string
}

interface BalancePoint {
	timestamp: number
	usdc: number
	usdt: number
	exotics: Record<string, number>
}

interface DashboardState {
	startTime: number
	ordersDetected: number
	ordersFilled: number
	ordersExecuted: number
	ordersSkipped: number
	recentActivity: ActivityEvent[]
	balances: Record<number, BalanceEntry>
	balanceHistory: BalancePoint[]
	hyperbridgeBalance: HyperbridgeBalance | null
}

export interface DashboardServiceOptions {
	monitor: EventMonitor
	bidStorage: BidStorageService | undefined
	chainClientManager: ChainClientManager
	configService: FillerConfigService
	fillerAddress: string
	curveConfig: DashboardConfig
	hyperbridgeWsUrl?: string
	substratePrivateKey?: string
	retractStaleBids?: () => Promise<number>
}

export class DashboardService {
	private server: Server
	private sseClients = new Set<ServerResponse>()
	private state: DashboardState
	private balanceRefreshInterval?: NodeJS.Timeout
	private logger = getLogger("dashboard")
	private options: DashboardServiceOptions
	// biome-ignore lint/suspicious/noExplicitAny: polkadot API type
	private polkadotApi?: any
	private substrateAddress?: string

	constructor(options: DashboardServiceOptions) {
		this.options = options
		// Restore persisted counters and balance history from SQLite
		const savedStats = options.bidStorage?.getDashboardStats() ?? {}
		const savedHistory = options.bidStorage?.getRecentBalanceHistory(200) ?? []

		this.state = {
			startTime: Date.now(),
			ordersDetected: savedStats["orders_detected"] ?? 0,
			ordersFilled: savedStats["orders_filled"] ?? 0,
			ordersExecuted: savedStats["orders_executed"] ?? 0,
			ordersSkipped: savedStats["orders_skipped"] ?? 0,
			recentActivity: [],
			balances: {},
			balanceHistory: savedHistory,
			hyperbridgeBalance: null,
		}

		this.server = createServer((req, res) => this.handleRequest(req, res))
		this.setupMonitorListeners()
	}

	start(port: number, host = "0.0.0.0"): void {
		this.server.listen(port, host, () => {
			this.logger.info({ bind: `${host}:${port}` }, `Simplex Dashboard running at http://localhost:${port}`)

			if (host === "0.0.0.0") {
				const localIp = getLocalNetworkIp()
				if (localIp) {
					this.logger.info(`Simplex Dashboard also reachable at http://${localIp}:${port}`)
				}
			}
		})

		// Initial balance fetch after 5s, then every 60s
		setTimeout(() => this.refreshBalances(), 5_000)
		this.balanceRefreshInterval = setInterval(() => this.refreshBalances(), 60_000)

		// Init polkadot API for Hyperbridge balance if configured
		if (this.options.hyperbridgeWsUrl && this.options.substratePrivateKey) {
			this.initPolkadotApi().catch((err) => {
				this.logger.warn({ err }, "Failed to initialize Polkadot API for Hyperbridge balance")
			})
		}
	}

	stop(): void {
		if (this.balanceRefreshInterval) clearInterval(this.balanceRefreshInterval)
		for (const client of this.sseClients) {
			try {
				client.end()
			} catch {}
		}
		this.sseClients.clear()
		if (this.polkadotApi) {
			this.polkadotApi.disconnect().catch(() => {})
		}
		this.server.close()
	}

	// ─── Polkadot / Hyperbridge Balance ──────────────────────────────────────────

	private async initPolkadotApi(): Promise<void> {
		// Dynamic import – polkadot is a heavy dep, only load when needed
		const { ApiPromise, WsProvider, Keyring } = await import("@polkadot/api")

		const provider = new WsProvider(this.options.hyperbridgeWsUrl!)
		this.polkadotApi = await ApiPromise.create({ provider })

		const keyring = new Keyring({ type: "sr25519" })
		const keypair = keyring.addFromUri(this.options.substratePrivateKey!)
		this.substrateAddress = keypair.address

		const fetchBalance = async () => {
			try {
				const account = await this.polkadotApi.query.system.account(this.substrateAddress)
				const decimals = (this.polkadotApi.registry.chainDecimals as number[])[0] ?? 12
				const symbol = (this.polkadotApi.registry.chainTokens as string[])[0] ?? "BRIDGE"

				const balance: HyperbridgeBalance = {
					address: this.substrateAddress!,
					free: account.data.free.toString(),
					reserved: account.data.reserved.toString(),
					decimals,
					symbol,
				}
				this.state.hyperbridgeBalance = balance
				this.broadcast("hyperbridge_balance", balance)
			} catch (err) {
				this.logger.warn({ err }, "Failed to fetch Hyperbridge balance")
			}
		}

		await fetchBalance()
		setInterval(fetchBalance, 60_000)
		this.logger.info({ address: this.substrateAddress }, "Hyperbridge balance tracking initialized")
	}

	// ─── Request Router ──────────────────────────────────────────────────────────

	private handleRequest(req: IncomingMessage, res: ServerResponse): void {
		const url = req.url ?? "/"
		const path = url.split("?")[0]

		res.setHeader("Access-Control-Allow-Origin", "*")

		if (path === "/events") {
			this.handleSSE(req, res)
		} else if (path === "/api/stats") {
			this.handleStats(res)
		} else if (path === "/api/bids") {
			this.handleBids(url, res)
		} else if (path === "/api/balances") {
			this.handleBalances(res)
		} else if (path === "/api/config") {
			this.handleConfig(res)
		} else if (path === "/api/retract-stale" && req.method === "POST") {
			this.handleRetractStale(res)
		} else if (path === "/api/balance-history") {
			this.handleBalanceHistory(url, res)
		} else if (path === "/" || path === "/index.html") {
			this.handleDashboard(res)
		} else {
			res.writeHead(404, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ error: "Not found" }))
		}
	}

	// ─── SSE ─────────────────────────────────────────────────────────────────────

	private handleSSE(req: IncomingMessage, res: ServerResponse): void {
		res.writeHead(200, {
			"Content-Type": "text/event-stream",
			"Cache-Control": "no-cache",
			Connection: "keep-alive",
			"X-Accel-Buffering": "no",
		})
		res.flushHeaders()

		this.sendSSE(res, "connected", { ok: true })
		this.sendSSE(res, "stats", this.buildStats())

		if (Object.keys(this.state.balances).length > 0) {
			this.sendSSE(res, "balances", this.state.balances)
		}

		if (this.state.balanceHistory.length > 0) {
			this.sendSSE(res, "balance_history", this.state.balanceHistory)
		}

		if (this.state.hyperbridgeBalance) {
			this.sendSSE(res, "hyperbridge_balance", this.state.hyperbridgeBalance)
		}

		for (const event of this.state.recentActivity.slice(-20)) {
			this.sendSSE(res, "activity", event)
		}

		this.sseClients.add(res)

		const ping = setInterval(() => {
			try {
				res.write(": ping\n\n")
			} catch {
				clearInterval(ping)
				this.sseClients.delete(res)
			}
		}, 15_000)

		req.on("close", () => {
			clearInterval(ping)
			this.sseClients.delete(res)
		})
	}

	private sendSSE(res: ServerResponse, event: string, data: unknown): void {
		try {
			res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
		} catch {}
	}

	private broadcast(event: string, data: unknown): void {
		const chunk = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`
		for (const client of this.sseClients) {
			try {
				client.write(chunk)
			} catch {
				this.sseClients.delete(client)
			}
		}
	}

	// ─── REST Handlers ────────────────────────────────────────────────────────────

	private handleDashboard(res: ServerResponse): void {
		const html = getDashboardHtml(this.options.curveConfig)
		res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" })
		res.end(html)
	}

	private handleStats(res: ServerResponse): void {
		res.writeHead(200, { "Content-Type": "application/json" })
		res.end(JSON.stringify(this.buildStats()))
	}

	private handleBids(url: string, res: ServerResponse): void {
		if (!this.options.bidStorage) {
			res.writeHead(200, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ bids: [], total: 0 }))
			return
		}

		const params = new URLSearchParams(url.split("?")[1] ?? "")
		const limit = Math.min(parseInt(params.get("limit") ?? "25"), 100)
		const offset = parseInt(params.get("offset") ?? "0")

		try {
			const stats = this.options.bidStorage.getStats()
			const bids = this.options.bidStorage.getBidsByDateRange(new Date(0), new Date())
			const page = bids.slice(offset, offset + limit)
			res.writeHead(200, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ bids: page, total: stats.total }))
		} catch {
			res.writeHead(500, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ error: "Failed to load bids" }))
		}
	}

	private handleBalances(res: ServerResponse): void {
		res.writeHead(200, { "Content-Type": "application/json" })
		res.end(JSON.stringify(this.state.balances))
	}

	private handleBalanceHistory(url: string, res: ServerResponse): void {
		res.writeHead(200, { "Content-Type": "application/json" })
		const params = new URLSearchParams(url.split("?")[1] ?? "")
		const since = parseInt(params.get("since") ?? "0") || 0

		if (this.options.bidStorage) {
			const history =
				since > 0
					? this.options.bidStorage.getBalanceHistorySince(since)
					: this.options.bidStorage.getRecentBalanceHistory(500)
			res.end(JSON.stringify(history))
		} else {
			const history =
				since > 0
					? this.state.balanceHistory.filter((p) => p.timestamp >= since)
					: this.state.balanceHistory
			res.end(JSON.stringify(history))
		}
	}

	private handleConfig(res: ServerResponse): void {
		res.writeHead(200, { "Content-Type": "application/json" })
		res.end(JSON.stringify(this.options.curveConfig))
	}

	private handleRetractStale(res: ServerResponse): void {
		if (!this.options.retractStaleBids) {
			res.writeHead(503, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ error: "Retraction not available (Hyperbridge not configured)" }))
			return
		}

		this.options
			.retractStaleBids()
			.then((queued) => {
				res.writeHead(200, { "Content-Type": "application/json" })
				res.end(JSON.stringify({ queued }))
				if (queued > 0) {
					this.trackActivity({
						type: "retract_sweep",
						timestamp: Date.now(),
						queued,
					})
					this.broadcast("activity", {
						type: "retract_sweep",
						timestamp: Date.now(),
						queued,
					})
				}
			})
			.catch((err) => {
				this.logger.error({ err }, "Retract stale bids failed")
				res.writeHead(500, { "Content-Type": "application/json" })
				res.end(JSON.stringify({ error: String(err) }))
			})
	}

	// ─── Stats Builder ────────────────────────────────────────────────────────────

	private buildStats() {
		const bidStats = this.options.bidStorage?.getStats() ?? {
			total: 0,
			successful: 0,
			failed: 0,
			retracted: 0,
			pendingRetraction: 0,
		}

		return {
			ordersDetected: this.state.ordersDetected,
			ordersFilled: this.state.ordersFilled,
			ordersExecuted: this.state.ordersExecuted,
			ordersSkipped: this.state.ordersSkipped,
			bidsTotal: bidStats.total,
			bidsSuccess: bidStats.successful,
			bidsFailed: bidStats.failed,
			bidsRetracted: bidStats.retracted,
			bidsPending: bidStats.pendingRetraction,
			uptimeMs: Date.now() - this.state.startTime,
		}
	}

	// ─── Monitor Listeners ────────────────────────────────────────────────────────

	private setupMonitorListeners(): void {
		const { monitor } = this.options

		monitor.on("newOrder", ({ order }) => {
			this.state.ordersDetected++
			this.options.bidStorage?.incrementDashboardStat("orders_detected")
			const event: ActivityEvent = {
				type: "order_detected",
				timestamp: Date.now(),
				orderId: order.id,
				source: order.source,
				destination: order.destination,
			}
			this.trackActivity(event)
			this.broadcast("activity", event)
			this.broadcast("stats", this.buildStats())
		})

		monitor.on("orderFilled", ({ orderId, hash }) => {
			this.state.ordersFilled++
			this.options.bidStorage?.incrementDashboardStat("orders_filled")
			const event: ActivityEvent = {
				type: "order_filled",
				timestamp: Date.now(),
				orderId,
				txHash: hash,
			}
			this.trackActivity(event)
			this.broadcast("activity", event)
			this.broadcast("stats", this.buildStats())
		})

		monitor.on("orderExecuted", ({ orderId, success, txHash, strategy, commitment, error }) => {
			this.state.ordersExecuted++
			this.options.bidStorage?.incrementDashboardStat("orders_executed")
			const event: ActivityEvent = {
				type: "order_executed",
				timestamp: Date.now(),
				orderId,
				success,
				txHash,
				strategy,
				commitment,
				error,
			}
			this.trackActivity(event)
			this.broadcast("activity", event)
			this.broadcast("stats", this.buildStats())

			if (commitment) {
				const bidEvent: ActivityEvent = {
					type: "bid_submitted",
					timestamp: Date.now(),
					commitment,
					success,
					error,
				}
				this.trackActivity(bidEvent)
				this.broadcast("activity", bidEvent)
			}
		})

		monitor.on("orderSkipped", ({ orderId, reason }) => {
			this.state.ordersSkipped++
			this.options.bidStorage?.incrementDashboardStat("orders_skipped")
			const event: ActivityEvent = {
				type: "order_skipped",
				timestamp: Date.now(),
				orderId,
				reason,
			}
			this.trackActivity(event)
			this.broadcast("activity", event)
		})

		monitor.on("orderDetected", ({ orderId }) => {
			const event: ActivityEvent = {
				type: "order_detected",
				timestamp: Date.now(),
				orderId,
				watchOnly: true,
			}
			this.trackActivity(event)
			this.broadcast("activity", event)
		})
	}

	private trackActivity(event: ActivityEvent): void {
		this.state.recentActivity.push(event)
		if (this.state.recentActivity.length > 200) {
			this.state.recentActivity = this.state.recentActivity.slice(-200)
		}
	}

	// ─── Balance History ──────────────────────────────────────────────────────────

	private computeBalancePoint(balances: Record<number, BalanceEntry>): BalancePoint {
		let usdc = 0
		let usdt = 0
		const exotics: Record<string, number> = {}
		for (const entry of Object.values(balances)) {
			if (entry.usdc) usdc += parseFloat(entry.usdc) || 0
			if (entry.usdt) usdt += parseFloat(entry.usdt) || 0
			for (const e of entry.exotics) {
				exotics[e.symbol] = (exotics[e.symbol] ?? 0) + (parseFloat(e.balance) || 0)
			}
		}
		return { timestamp: Date.now(), usdc, usdt, exotics }
	}

	// ─── Balance Refresh ──────────────────────────────────────────────────────────

	private async refreshBalances(): Promise<void> {
		const chainIds = this.options.configService.getConfiguredChainIds()

		// Collect FX strategy exotic token addresses keyed by chain ID
		const fxExoticByChain = new Map<number, string>()
		for (const strategy of this.options.curveConfig.strategies) {
			if (strategy.type === "hyperfx") {
				for (const [chainKey, addr] of Object.entries(strategy.exoticTokenAddresses)) {
					const id = parseInt(chainKey.replace("EVM-", ""), 10)
					if (!isNaN(id)) fxExoticByChain.set(id, addr)
				}
			}
		}

		const updated: Record<number, BalanceEntry> = {}

		await Promise.allSettled(
			chainIds.map(async (chainId) => {
				const chain = `EVM-${chainId}`
				const client = this.options.chainClientManager.getPublicClient(chain)
				const fillerAddr = this.options.fillerAddress as `0x${string}`

				const entry: BalanceEntry = {
					usdc: null,
					usdt: null,
					native: null,
					nativeSymbol: CHAIN_NATIVE_SYMBOLS[chainId] ?? "ETH",
					exotics: [],
				}

				// Native balance
				try {
					const native = await client.getBalance({ address: fillerAddr })
					entry.native = formatUnits(native, 18)
				} catch {}

				// USDC
				try {
					const usdcAddr = this.options.configService.getUsdcAsset(chain)
					const usdcDecimals = this.options.configService.getUsdcDecimals(chain)
					const balance = await client.readContract({
						address: usdcAddr as `0x${string}`,
						abi: ERC20_ABI,
						functionName: "balanceOf",
						args: [fillerAddr],
					})
					entry.usdc = formatUnits(balance as bigint, usdcDecimals)
				} catch {}

				// USDT
				try {
					const usdtAddr = this.options.configService.getUsdtAsset(chain)
					const usdtDecimals = this.options.configService.getUsdtDecimals(chain)
					const balance = await client.readContract({
						address: usdtAddr as `0x${string}`,
						abi: ERC20_ABI,
						functionName: "balanceOf",
						args: [fillerAddr],
					})
					entry.usdt = formatUnits(balance as bigint, usdtDecimals)
				} catch {}

				// Exotic tokens from FX strategy config
				const fxAddr = fxExoticByChain.get(chainId)
				if (fxAddr) {
					try {
						let symbol = "EXOTIC"
						try {
							symbol = (await client.readContract({
								address: fxAddr as `0x${string}`,
								abi: ERC20_ABI,
								functionName: "symbol",
								args: [],
							})) as string
						} catch {}
						let decimals = 18
						try {
							decimals = (await client.readContract({
								address: fxAddr as `0x${string}`,
								abi: ERC20_ABI,
								functionName: "decimals",
								args: [],
							})) as number
						} catch {}
						const balance = await client.readContract({
							address: fxAddr as `0x${string}`,
							abi: ERC20_ABI,
							functionName: "balanceOf",
							args: [fillerAddr],
						})
						entry.exotics.push({ symbol, balance: formatUnits(balance as bigint, decimals) })
					} catch {}
				}

				updated[chainId] = entry
			}),
		)

		this.state.balances = updated
		this.broadcast("balances", updated)

		const point = this.computeBalancePoint(updated)
		this.options.bidStorage?.insertBalancePoint(point)
		this.state.balanceHistory.push(point)
		if (this.state.balanceHistory.length > 200) {
			this.state.balanceHistory = this.state.balanceHistory.slice(-200)
		}
		this.broadcast("balance_point", point)

		this.logger.debug({ chains: chainIds.length }, "Balances refreshed for dashboard")
	}
}
