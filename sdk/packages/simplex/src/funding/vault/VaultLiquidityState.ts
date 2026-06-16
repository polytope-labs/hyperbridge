import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { VaultConfig, HydratedVault } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { getLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"
import { parseUnits } from "viem"

const logger = getLogger("vault-state")

/** Default dust guard (absolute human units) when a vault omits `minSweep`. */
const DEFAULT_MIN_SWEEP = "10"

/**
 * Backstop expiry for a reservation whose bid never executes (lost auction,
 * abandoned) — without it, `remaining` would drift to 0 and disable sourcing.
 * Won bids release immediately via position-decrease reconciliation in
 * {@link refresh}, so this only needs to exceed plan→execute latency (~15s
 * auction + settlement); expiring too early risks an oversubscribed withdraw.
 */
const RESERVATION_TTL_MS = 30_000

/**
 * Long-lived vault (ERC-4626) liquidity state for one destination chain.
 *
 * Each configured vault maps its underlying asset (e.g. USDC) to the solver's
 * position. The sourceable amount is `vault.maxWithdraw(solver)` — the vault's
 * own answer covering both the solver's balance and any liquidity constraint
 * (e.g. Aave utilization for stataTokens). Planning within it avoids a
 * `withdraw` that would revert and roll back the entire ERC-7821 fill batch.
 *
 * Concurrent access is serialised by the planner's per-chain mutex.
 */
export class VaultLiquidityState {
	/** Keyed by underlying asset address, lowercased. */
	private vaults = new Map<string, HydratedVault>()
	private hydrated = false
	/**
	 * Per-asset reservations for planned-but-unexecuted withdrawals, lowercased
	 * key. Each reservation auto-expires after {@link RESERVATION_TTL_MS} so a
	 * lost bid's reservation cannot accumulate and starve `remaining`.
	 */
	private reservations = new Map<string, { amount: bigint; expiresAt: number }[]>()
	/** Last observed position in asset terms, used to release reservations once
	 * their withdrawal actually executes on-chain (the position drops). */
	private lastPositionAssets = new Map<string, bigint>()

	constructor(
		private readonly chain: string,
		private readonly configs: VaultConfig[],
		private readonly solver: HexString,
		private readonly clientManager: ChainClientManager,
	) {}

	// =========================================================================
	// Initialisation & refresh
	// =========================================================================

	/**
	 * One-time hydration: resolves each vault's underlying asset and decimals,
	 * then loads live balances via {@link refresh}.
	 */
	async hydrate(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const cfg of this.configs) {
			const asset = (await client.readContract({
				address: cfg.vault,
				abi: ERC4626_ABI,
				functionName: "asset",
			})) as HexString

			if (!asset || asset === "0x0000000000000000000000000000000000000000") {
				throw new Error(`Vault ${cfg.vault} on ${this.chain} has no underlying asset`)
			}

			const decimals = (await client.readContract({
				address: asset,
				abi: ERC20_ABI,
				functionName: "decimals",
			})) as number

			this.vaults.set(asset.toLowerCase(), {
				vault: cfg.vault,
				asset,
				decimals,
				thresholdScaled: cfg.threshold ? parseUnits(cfg.threshold, decimals) : null,
				minSweepScaled: parseUnits(cfg.minSweep ?? DEFAULT_MIN_SWEEP, decimals),
				redeemOnShutdown: cfg.redeemOnShutdown ?? true,
				positionAssets: 0n,
				maxWithdrawable: 0n,
				remaining: 0n,
			})
		}

		await this.refresh()
		this.hydrated = true

		for (const v of this.vaults.values()) {
			logger.info(
				{
					chain: this.chain,
					vault: v.vault,
					asset: v.asset,
					decimals: v.decimals,
					positionAssets: v.positionAssets.toString(),
					maxWithdrawable: v.maxWithdrawable.toString(),
				},
				"Vault hydrated",
			)
		}
		logger.info({ chain: this.chain, vaults: this.configs.length }, "Vault liquidity state hydrated")
	}

	/**
	 * Refreshes live balances for every vault: the solver's position in asset
	 * terms and the vault's withdraw cap. Reconciles the per-asset `consumed`
	 * counter against any position decrease since the last refresh so executed
	 * withdrawals free up their reservation.
	 */
	async refresh(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const v of this.vaults.values()) {
			const key = v.asset.toLowerCase()

			const shares = (await client.readContract({
				address: v.vault,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [this.solver],
			})) as bigint

			const [positionAssets, maxWithdrawable] = await Promise.all([
				client.readContract({
					address: v.vault,
					abi: ERC4626_ABI,
					functionName: "previewRedeem",
					args: [shares],
				}) as Promise<bigint>,
				client.readContract({
					address: v.vault,
					abi: ERC4626_ABI,
					functionName: "maxWithdraw",
					args: [this.solver],
				}) as Promise<bigint>,
			])

			// A drop in the on-chain position means a planned withdrawal has
			// executed — release that much reservation immediately (oldest first)
			// so the freed liquidity is available to the next fill without waiting
			// for the TTL. The TTL only backstops bids that never execute.
			const prevPosition = this.lastPositionAssets.get(key) ?? positionAssets
			const decrease = prevPosition > positionAssets ? prevPosition - positionAssets : 0n
			this.releaseReserved(key, decrease)
			this.lastPositionAssets.set(key, positionAssets)

			// Subtract only live reservations; expired ones (e.g. from lost bids)
			// are pruned so they can never permanently shrink `remaining`.
			const reserved = this.reservedFor(key)

			v.positionAssets = positionAssets
			v.maxWithdrawable = maxWithdrawable
			v.remaining = maxWithdrawable > reserved ? maxWithdrawable - reserved : 0n

			logger.debug(
				{
					chain: this.chain,
					vault: v.vault,
					asset: v.asset,
					positionAssets: positionAssets.toString(),
					maxWithdrawable: maxWithdrawable.toString(),
					reserved: reserved.toString(),
					remaining: v.remaining.toString(),
				},
				"Vault refreshed",
			)
		}
	}

	// =========================================================================
	// Lookups & accounting
	// =========================================================================

	isHydrated(): boolean {
		return this.hydrated
	}

	allVaults(): HydratedVault[] {
		return Array.from(this.vaults.values())
	}

	/** Vault whose underlying asset matches `tokenLower`, if configured. */
	vaultForToken(tokenLower: string): HydratedVault | undefined {
		return this.vaults.get(tokenLower.toLowerCase())
	}

	/** Sourceable amount of `asset` after pending-fill reservations. */
	remaining(asset: string): bigint {
		return this.vaults.get(asset.toLowerCase())?.remaining ?? 0n
	}

	consume(asset: string, amount: bigint): void {
		const key = asset.toLowerCase()
		const list = this.reservations.get(key) ?? []
		list.push({ amount, expiresAt: Date.now() + RESERVATION_TTL_MS })
		this.reservations.set(key, list)

		const v = this.vaults.get(key)
		if (v) {
			v.remaining = v.remaining > amount ? v.remaining - amount : 0n
		}
	}

	/**
	 * Removes up to `amount` of reservations (oldest first) to reflect a realised
	 * on-chain position decrease — i.e. a planned withdrawal that has executed.
	 */
	private releaseReserved(key: string, amount: bigint): void {
		if (amount <= 0n) return
		const list = this.reservations.get(key)
		if (!list || list.length === 0) return

		let toRelease = amount
		while (toRelease > 0n && list.length > 0) {
			const head = list[0]
			if (head.amount <= toRelease) {
				toRelease -= head.amount
				list.shift()
			} else {
				head.amount -= toRelease
				toRelease = 0n
			}
		}
		this.reservations.set(key, list)
	}

	/** Sum of unexpired reservations for `key`, pruning expired ones in place. */
	private reservedFor(key: string): bigint {
		const list = this.reservations.get(key)
		if (!list || list.length === 0) return 0n

		const now = Date.now()
		const live = list.filter((r) => r.expiresAt > now)
		if (live.length !== list.length) this.reservations.set(key, live)

		return live.reduce((sum, r) => sum + r.amount, 0n)
	}
}
