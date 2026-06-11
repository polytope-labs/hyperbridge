import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { Erc4626VaultConfig, HydratedErc4626Vault } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { getLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"
import { parseUnits } from "viem"

const logger = getLogger("erc4626-state")

/** Default dust guard (absolute human units) when a vault omits `minSweep`. */
const DEFAULT_MIN_SWEEP = "10"

/**
 * Long-lived ERC-4626 liquidity state for one destination chain.
 *
 * Each configured vault maps its underlying asset (e.g. USDC) to the solver's
 * position. The sourceable amount is `vault.maxWithdraw(solver)` — the vault's
 * own answer covering both the solver's balance and any liquidity constraint
 * (e.g. Aave utilization for stataTokens). Planning within it avoids a
 * `withdraw` that would revert and roll back the entire ERC-7821 fill batch.
 *
 * Concurrent access is serialised by the planner's per-chain mutex.
 */
export class Erc4626LiquidityState {
	/** Keyed by underlying asset address, lowercased. */
	private vaults = new Map<string, HydratedErc4626Vault>()
	private hydrated = false
	/** Per-asset amount reserved for in-flight fills this round, lowercased key. */
	private consumed = new Map<string, bigint>()
	/** Last observed position in asset terms, used to reconcile `consumed`. */
	private lastPositionAssets = new Map<string, bigint>()

	constructor(
		private readonly chain: string,
		private readonly configs: Erc4626VaultConfig[],
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
				throw new Error(`ERC-4626 vault ${cfg.vault} on ${this.chain} has no underlying asset`)
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
				"ERC-4626 vault hydrated",
			)
		}
		logger.info({ chain: this.chain, vaults: this.configs.length }, "ERC-4626 liquidity state hydrated")
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

			// Reconcile consumed against realised position decrease.
			const prevPosition = this.lastPositionAssets.get(key) ?? positionAssets
			const decrease = prevPosition > positionAssets ? prevPosition - positionAssets : 0n
			const prevConsumed = this.consumed.get(key) ?? 0n
			const newConsumed = prevConsumed > decrease ? prevConsumed - decrease : 0n
			this.consumed.set(key, newConsumed)
			this.lastPositionAssets.set(key, positionAssets)

			v.positionAssets = positionAssets
			v.maxWithdrawable = maxWithdrawable
			v.remaining = maxWithdrawable > newConsumed ? maxWithdrawable - newConsumed : 0n

			logger.debug(
				{
					chain: this.chain,
					vault: v.vault,
					asset: v.asset,
					positionAssets: positionAssets.toString(),
					maxWithdrawable: maxWithdrawable.toString(),
					consumed: newConsumed.toString(),
					remaining: v.remaining.toString(),
				},
				"ERC-4626 vault refreshed",
			)
		}
	}

	// =========================================================================
	// Lookups & accounting
	// =========================================================================

	isHydrated(): boolean {
		return this.hydrated
	}

	allVaults(): HydratedErc4626Vault[] {
		return Array.from(this.vaults.values())
	}

	/** Vault whose underlying asset matches `tokenLower`, if configured. */
	vaultForToken(tokenLower: string): HydratedErc4626Vault | undefined {
		return this.vaults.get(tokenLower.toLowerCase())
	}

	/** Sourceable amount of `asset` after pending-fill reservations. */
	remaining(asset: string): bigint {
		return this.vaults.get(asset.toLowerCase())?.remaining ?? 0n
	}

	consume(asset: string, amount: bigint): void {
		const key = asset.toLowerCase()
		const v = this.vaults.get(key)
		if (v) {
			v.remaining = v.remaining > amount ? v.remaining - amount : 0n
		}
		this.consumed.set(key, (this.consumed.get(key) ?? 0n) + amount)
	}
}
