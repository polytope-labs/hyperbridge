import { formatUnits } from "viem"
import type { IntentsCoprocessor } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { VaultBalancePosition } from "@/funding/types"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { moduleLogger, type Logger } from "./Logger"
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
	exotics?: Array<{ symbol: string; amount: number }>
	/** Canonical token balances, including funds held in configured ERC-4626 vaults. */
	assets: AssetBalanceRow[]
}

export type AssetBalanceStatus = "fresh" | "partial" | "unavailable"

export interface VaultBalanceRow {
	address: string
	position: number
	available: number
}

export interface AssetBalanceRow {
	address: string
	symbol: string
	/** Direct underlying balance held by the solver account. Null when its RPC read failed. */
	wallet: number | null
	/** Configured wallet floor that must remain liquid. */
	walletReserve: number
	/** Total solver-owned underlying represented by vault shares. */
	vaultPosition: number
	/** Vault assets that can be withdrawn now, after pending-fill reservations. */
	vaultAvailable: number
	/** Wallet plus vault position. Null when any required source was unavailable. */
	total: number | null
	/** Spendable wallet above reserve plus currently withdrawable vault assets. */
	available: number | null
	vaults: VaultBalanceRow[]
	status: AssetBalanceStatus
}

export type BalanceIssueSource = "chain" | "native" | "token" | "vault"

export interface BalanceIssue {
	chainId?: number
	source: BalanceIssueSource
	asset?: string
	message: string
}

export type BalanceSnapshotStatus = "loading" | "fresh" | "partial" | "unavailable"

export interface HyperbridgeBalance {
	address: string
	free: number
	reserved: number
}

export interface BalanceSnapshot {
	/** null until the first refresh attempt completes. */
	updatedAt: number | null
	status: BalanceSnapshotStatus
	chains: ChainBalanceRow[]
	issues: BalanceIssue[]
	hyperbridge?: HyperbridgeBalance
}

/** Narrow read-only seam implemented by the vault funding planner. */
export interface VaultBalanceSource {
	getBalanceSnapshot(chain?: string): Promise<VaultBalancePosition[]>
}

export interface BalanceProviderOptions {
	chainClientManager: ChainClientManager
	configService: FillerConfigService
	fillerAddress: string
	/** Exotic token addresses per state machine id — every cross-asset pair's token1 on that chain. */
	token1: Record<string, string[]>
	/**
	 * The filler's Hyperbridge connection, reused rather than dialled again. A second socket to the
	 * same node doubles the reconnect traffic during an outage for a balance read that is only
	 * needed once a minute.
	 */
	hyperbridge?: Promise<IntentsCoprocessor>
	substratePrivateKey?: string
	refreshIntervalMs?: number
	vaultBalances?: VaultBalanceSource
}

/**
 * Periodically collects wallet balances (native, USDC, USDT, exotic per chain,
 * plus the BRIDGE balance of the substrate account) into a plain snapshot,
 * consumed by the UI JSON API and `wallet.balances()`.
 */
export class BalanceProvider {
	private snapshot: BalanceSnapshot = { updatedAt: null, status: "loading", chains: [], issues: [] }
	private stopped = false
	private refreshInterval?: NodeJS.Timeout
	private hyperbridgeInterval?: NodeJS.Timeout
	private logger: Logger
	private options: BalanceProviderOptions
	private intervalMs: number

	constructor(options: BalanceProviderOptions) {
		this.logger = moduleLogger(options.configService.loggers, "balances")
		this.options = options
		this.intervalMs = options.refreshIntervalMs ?? 60_000
	}

	async start(): Promise<void> {
		this.stopped = false
		await this.refresh()
		if (this.stopped) return
		this.refreshInterval = setInterval(() => void this.refresh(), this.intervalMs)

		if (this.options.hyperbridge && this.options.substratePrivateKey) {
			this.trackHyperbridgeBalance().catch((err) => {
				this.logger.warn({ err }, "Failed to start Hyperbridge balance tracking")
			})
		}
	}

	stop(): void {
		this.stopped = true
		if (this.refreshInterval) clearInterval(this.refreshInterval)
		if (this.hyperbridgeInterval) clearInterval(this.hyperbridgeInterval)
		// The Hyperbridge connection belongs to the filler; it is not ours to close.
	}

	getSnapshot(): BalanceSnapshot {
		return this.snapshot
	}

	async refresh(): Promise<BalanceSnapshot> {
		const chainIds = this.options.configService.getConfiguredChainIds()
		const issues: BalanceIssue[] = []
		const vaultPositionsByChain = new Map<number, VaultBalancePosition[]>()
		const unavailableVaultChains = new Set<number>()

		if (this.options.vaultBalances) {
			await Promise.all(
				chainIds.map(async (chainId) => {
					try {
						const positions = await this.options.vaultBalances!.getBalanceSnapshot(`EVM-${chainId}`)
						vaultPositionsByChain.set(chainId, positions)
					} catch (err) {
						unavailableVaultChains.add(chainId)
						issues.push({ chainId, source: "vault", message: errorMessage(err) })
						this.logger.warn({ err, chainId }, "Failed to refresh vault balances")
					}
				}),
			)
		}

		const fxExoticByChain = new Map<number, string[]>()
		for (const [chainKey, addrs] of Object.entries(this.options.token1)) {
			const id = Number.parseInt(chainKey.replace("EVM-", ""), 10)
			if (!isNaN(id)) fxExoticByChain.set(id, addrs)
		}

		const results = await Promise.all(
			chainIds.map(async (chainId) => {
				try {
					return await this.collectChain(
						chainId,
						fxExoticByChain,
						vaultPositionsByChain.get(chainId) ?? [],
						!unavailableVaultChains.has(chainId),
					)
				} catch (err) {
					return {
						row: { chainId, assets: [] },
						issues: [{ chainId, source: "chain" as const, message: errorMessage(err) }],
					}
				}
			}),
		)
		const rows = results.map((result) => result.row)
		issues.push(...results.flatMap((result) => result.issues))
		const hasData =
			Array.from(vaultPositionsByChain.values()).some((positions) => positions.length > 0) ||
			rows.some((row) => row.native !== undefined || row.assets.some((asset) => asset.wallet !== null))

		this.snapshot = {
			updatedAt: Date.now(),
			status: issues.length === 0 ? "fresh" : hasData ? "partial" : "unavailable",
			chains: rows,
			issues,
			hyperbridge: this.snapshot.hyperbridge,
		}
		this.logger.debug({ chains: chainIds.length }, "Balances refreshed")
		return this.snapshot
	}

	private async collectChain(
		chainId: number,
		fxExoticByChain: Map<number, string[]>,
		vaultPositions: VaultBalancePosition[],
		vaultDataAvailable: boolean,
	): Promise<{ row: ChainBalanceRow; issues: BalanceIssue[] }> {
		const chain = `EVM-${chainId}`
		const client = this.options.chainClientManager.getPublicClient(chain)
		const fillerAddr = this.options.fillerAddress as `0x${string}`
		const row: ChainBalanceRow = { chainId, assets: [] }
		const issues: BalanceIssue[] = []

		try {
			const native = await client.getBalance({ address: fillerAddr })
			const symbol = CHAIN_NATIVE_SYMBOLS[chainId] ?? "ETH"
			row.native = { symbol, amount: Number.parseFloat(formatUnits(native, 18)) }
		} catch (err) {
			issues.push({ chainId, source: "native", message: errorMessage(err) })
		}

		const definitions = new Map<string, TokenDefinition>()
		this.addConfiguredStable(definitions, chainId, "USDC", () => ({
			address: this.options.configService.getUsdcAsset(chain),
			decimals: this.options.configService.getUsdcDecimals(chain),
		}), issues)
		this.addConfiguredStable(definitions, chainId, "USDT", () => ({
			address: this.options.configService.getUsdtAsset(chain),
			decimals: this.options.configService.getUsdtDecimals(chain),
		}), issues)

		for (const fxAddr of new Set(fxExoticByChain.get(chainId) ?? [])) {
			if (!isConfiguredAddress(fxAddr)) {
				if (!isUnsetAddress(fxAddr)) {
					issues.push({ chainId, source: "token", asset: fxAddr, message: "Invalid token address" })
				}
				continue
			}
			if (definitions.has(fxAddr.toLowerCase())) continue

			let symbol = "EXOTIC"
			let decimals: number | null = null
			try {
				symbol = (await client.readContract({
					address: fxAddr as `0x${string}`,
					abi: ERC20_ABI,
					functionName: "symbol",
					args: [],
				})) as string
			} catch (err) {
				issues.push({ chainId, source: "token", asset: fxAddr, message: `Symbol read failed: ${errorMessage(err)}` })
			}
			try {
				decimals = (await client.readContract({
					address: fxAddr as `0x${string}`,
					abi: ERC20_ABI,
					functionName: "decimals",
					args: [],
				})) as number
			} catch (err) {
				issues.push({ chainId, source: "token", asset: fxAddr, message: `Decimals read failed: ${errorMessage(err)}` })
			}
			definitions.set(fxAddr.toLowerCase(), { address: fxAddr, symbol, decimals, kind: "exotic" })
		}

		for (const position of vaultPositions) {
			const key = position.asset.toLowerCase()
			const existing = definitions.get(key)
			if (existing) {
				if (existing.decimals === null) existing.decimals = position.decimals
				if (existing.symbol === "EXOTIC") existing.symbol = position.symbol
			} else {
				definitions.set(key, {
					address: position.asset,
					symbol: position.symbol,
					decimals: position.decimals,
					kind: "vault",
				})
			}
		}

		row.assets = await Promise.all(
			Array.from(definitions.values()).map(async (definition) => {
				let wallet: number | null = null
				if (definition.decimals !== null) {
					try {
						const balance = await client.readContract({
							address: definition.address as `0x${string}`,
							abi: ERC20_ABI,
							functionName: "balanceOf",
							args: [fillerAddr],
						})
						wallet = Number.parseFloat(formatUnits(balance as bigint, definition.decimals))
					} catch (err) {
						issues.push({
							chainId,
							source: "token",
							asset: definition.symbol,
							message: errorMessage(err),
						})
					}
				}

				const positions = vaultPositions.filter(
					(position) => position.asset.toLowerCase() === definition.address.toLowerCase(),
				)
				const walletReserve = sumFormatted(positions, "walletReserve")
				const vaultPosition = sumFormatted(positions, "positionAssets")
				const vaultAvailable = sumFormatted(positions, "availableAssets")
				const complete = wallet !== null && vaultDataAvailable
				const asset: AssetBalanceRow = {
					address: definition.address,
					symbol: definition.symbol,
					wallet,
					walletReserve,
					vaultPosition,
					vaultAvailable,
					total: wallet !== null && vaultDataAvailable ? wallet + vaultPosition : null,
					available:
						wallet !== null && vaultDataAvailable ? Math.max(wallet - walletReserve, 0) + vaultAvailable : null,
					vaults: positions.map((position) => ({
						address: position.vault,
						position: Number.parseFloat(formatUnits(position.positionAssets, position.decimals)),
						available: Number.parseFloat(formatUnits(position.availableAssets, position.decimals)),
					})),
					status: complete ? "fresh" : wallet !== null || positions.length > 0 ? "partial" : "unavailable",
				}

				if (definition.kind === "usdc" && wallet !== null) row.usdc = wallet
				if (definition.kind === "usdt" && wallet !== null) row.usdt = wallet
				if (definition.kind === "exotic" && wallet !== null) {
					;(row.exotics ??= []).push({ symbol: definition.symbol, amount: wallet })
				}
				return asset
			}),
		)

		return { row, issues }
	}

	private addConfiguredStable(
		definitions: Map<string, TokenDefinition>,
		chainId: number,
		symbol: "USDC" | "USDT",
		resolve: () => { address: string; decimals: number },
		issues: BalanceIssue[],
	): void {
		try {
			const { address, decimals } = resolve()
			if (isUnsetAddress(address)) return
			if (!isConfiguredAddress(address)) {
				issues.push({ chainId, source: "token", asset: symbol, message: "Invalid token address" })
				return
			}
			definitions.set(address.toLowerCase(), {
				address,
				symbol,
				decimals,
				kind: symbol.toLowerCase() as "usdc" | "usdt",
			})
		} catch (err) {
			issues.push({ chainId, source: "token", asset: symbol, message: errorMessage(err) })
		}
	}

	private async trackHyperbridgeBalance(): Promise<void> {
		const coprocessor = await this.options.hyperbridge!
		const keypair = await deriveSubstrateKeyPair(this.options.substratePrivateKey!)
		const address = keypair.address

		const fetchBalance = async () => {
			try {
				// Queried over HTTP, so a websocket outage does not stall the balance read; a failed
				// request leaves the last snapshot in place and the next tick picks it up.
				const api = await coprocessor.queryApi()
				const account = (await api.query.system.account(address)) as any
				const decimals = (api.registry.chainDecimals as number[])[0] ?? 12

				const free = Number.parseFloat(formatUnits(BigInt(account.data.free.toString()), decimals))
				const reserved = Number.parseFloat(formatUnits(BigInt(account.data.reserved.toString()), decimals))

				this.snapshot = { ...this.snapshot, hyperbridge: { address, free, reserved } }
			} catch (err) {
				this.logger.warn({ err }, "Failed to fetch Hyperbridge balance")
			}
		}

		await fetchBalance()
		// stop() may have run while the awaits above were in flight; a timer
		// installed now would outlive the provider.
		if (this.stopped) return
		this.hyperbridgeInterval = setInterval(fetchBalance, this.intervalMs)
		this.logger.info({ address }, "Hyperbridge balance tracking initialized")
	}
}

interface TokenDefinition {
	address: string
	symbol: string
	decimals: number | null
	kind: "usdc" | "usdt" | "exotic" | "vault"
}

function isUnsetAddress(address: string): boolean {
	return /^0x0*$/i.test(address)
}

function isConfiguredAddress(address: string): boolean {
	return /^0x[0-9a-f]{40}$/i.test(address) && !isUnsetAddress(address)
}

function sumFormatted(
	positions: VaultBalancePosition[],
	field: "walletReserve" | "positionAssets" | "availableAssets",
): number {
	return positions.reduce(
		(total, position) => total + Number.parseFloat(formatUnits(position[field], position.decimals)),
		0,
	)
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}
