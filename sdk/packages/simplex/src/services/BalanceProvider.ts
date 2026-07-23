import { formatUnits } from "viem"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import { deriveSubstrateKeyPair } from "./substrate-key"

export const CHAIN_NATIVE_SYMBOLS: Record<number, string> = {
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

export interface ChainBalanceRow {
	chainId: number
	native?: { symbol: string; amount: number }
	usdc?: number
	usdt?: number
	exotic?: { symbol: string; amount: number }
}

export interface HyperbridgeBalance {
	address: string
	free: number
	reserved: number
}

export interface BalanceSnapshot {
	/** null until the first successful refresh */
	updatedAt: number | null
	chains: ChainBalanceRow[]
	hyperbridge?: HyperbridgeBalance
}

export interface BalanceProviderOptions {
	chainClientManager: ChainClientManager
	configService: FillerConfigService
	fillerAddress: string
	/** Exotic token address per state machine id, merged across hyperfx strategies. */
	token1: Record<string, string>
	hyperbridgeWsUrl?: string
	substratePrivateKey?: string
	refreshIntervalMs?: number
}

/**
 * Periodically collects wallet balances (native, USDC, USDT, exotic per chain,
 * plus the BRIDGE balance of the substrate account) into a plain snapshot,
 * consumed by both the Prometheus gauges and the UI JSON API.
 */
export class BalanceProvider {
	private snapshot: BalanceSnapshot = { updatedAt: null, chains: [] }
	private initialTimeout?: NodeJS.Timeout
	private refreshInterval?: NodeJS.Timeout
	private hyperbridgeInterval?: NodeJS.Timeout
	// biome-ignore lint/suspicious/noExplicitAny: polkadot API type
	private polkadotApi?: any
	private logger = getLogger("balances")
	private options: BalanceProviderOptions
	private intervalMs: number

	constructor(options: BalanceProviderOptions) {
		this.options = options
		this.intervalMs = options.refreshIntervalMs ?? 60_000
	}

	start(): void {
		this.initialTimeout = setTimeout(() => void this.refresh(), 5_000)
		this.refreshInterval = setInterval(() => void this.refresh(), this.intervalMs)

		if (this.options.hyperbridgeWsUrl && this.options.substratePrivateKey) {
			this.initPolkadotApi().catch((err) => {
				this.logger.warn({ err }, "Failed to initialize Polkadot API for Hyperbridge balance")
			})
		}
	}

	stop(): void {
		if (this.initialTimeout) clearTimeout(this.initialTimeout)
		if (this.refreshInterval) clearInterval(this.refreshInterval)
		if (this.hyperbridgeInterval) clearInterval(this.hyperbridgeInterval)
		if (this.polkadotApi) {
			this.polkadotApi.disconnect().catch(() => {})
			this.polkadotApi = undefined
		}
	}

	getSnapshot(): BalanceSnapshot {
		return this.snapshot
	}

	async refresh(): Promise<BalanceSnapshot> {
		const chainIds = this.options.configService.getConfiguredChainIds()

		const fxExoticByChain = new Map<number, string>()
		for (const [chainKey, addr] of Object.entries(this.options.token1)) {
			const id = parseInt(chainKey.replace("EVM-", ""), 10)
			if (!isNaN(id)) fxExoticByChain.set(id, addr)
		}

		const rows = await Promise.all(chainIds.map((chainId) => this.collectChain(chainId, fxExoticByChain)))

		this.snapshot = {
			updatedAt: Date.now(),
			chains: rows,
			hyperbridge: this.snapshot.hyperbridge,
		}
		this.logger.debug({ chains: chainIds.length }, "Balances refreshed")
		return this.snapshot
	}

	private async collectChain(chainId: number, fxExoticByChain: Map<number, string>): Promise<ChainBalanceRow> {
		const chain = `EVM-${chainId}`
		const client = this.options.chainClientManager.getPublicClient(chain)
		const fillerAddr = this.options.fillerAddress as `0x${string}`
		const row: ChainBalanceRow = { chainId }

		try {
			const native = await client.getBalance({ address: fillerAddr })
			const symbol = CHAIN_NATIVE_SYMBOLS[chainId] ?? "ETH"
			row.native = { symbol, amount: parseFloat(formatUnits(native, 18)) }
		} catch {}

		try {
			const usdcAddr = this.options.configService.getUsdcAsset(chain)
			const usdcDecimals = this.options.configService.getUsdcDecimals(chain)
			const balance = await client.readContract({
				address: usdcAddr as `0x${string}`,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [fillerAddr],
			})
			row.usdc = parseFloat(formatUnits(balance as bigint, usdcDecimals))
		} catch {}

		try {
			const usdtAddr = this.options.configService.getUsdtAsset(chain)
			const usdtDecimals = this.options.configService.getUsdtDecimals(chain)
			const balance = await client.readContract({
				address: usdtAddr as `0x${string}`,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [fillerAddr],
			})
			row.usdt = parseFloat(formatUnits(balance as bigint, usdtDecimals))
		} catch {}

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
				row.exotic = { symbol, amount: parseFloat(formatUnits(balance as bigint, decimals)) }
			} catch {}
		}

		return row
	}

	private async initPolkadotApi(): Promise<void> {
		const { ApiPromise, WsProvider } = await import("@polkadot/api")

		const provider = new WsProvider(this.options.hyperbridgeWsUrl!)
		this.polkadotApi = await ApiPromise.create({ provider })

		const keypair = await deriveSubstrateKeyPair(this.options.substratePrivateKey!)
		const address = keypair.address

		const fetchBalance = async () => {
			try {
				const account = await this.polkadotApi.query.system.account(address)
				const decimals = (this.polkadotApi.registry.chainDecimals as number[])[0] ?? 12

				const free = parseFloat(formatUnits(BigInt(account.data.free.toString()), decimals))
				const reserved = parseFloat(formatUnits(BigInt(account.data.reserved.toString()), decimals))

				this.snapshot = { ...this.snapshot, hyperbridge: { address, free, reserved } }
			} catch (err) {
				this.logger.warn({ err }, "Failed to fetch Hyperbridge balance")
			}
		}

		await fetchBalance()
		this.hyperbridgeInterval = setInterval(fetchBalance, this.intervalMs)
		this.logger.info({ address }, "Hyperbridge balance tracking initialized")
	}
}
