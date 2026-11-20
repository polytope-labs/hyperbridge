import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { VaultConfig, HydratedVault } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { type Logger, moduleLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"
import { parseUnits } from "viem"

/**
 * Long-lived vault (ERC-4626) liquidity state for one destination chain.
 *
 * Each configured vault maps its underlying asset (e.g. USDC) to the solver's
 * position. The sourceable amount is `vault.maxWithdraw(solver)` — the vault's
 * own answer covering both the solver's balance and any liquidity constraint
 * (e.g. Aave utilization for stataTokens). Planning within it avoids a
 * `withdraw` that would revert and roll back the entire ERC-7821 fill batch.
 *
 * Nothing is held back locally for withdrawals that are planned but not yet
 * executed: the chain is the only record of what is left. Two bids planned
 * back to back can each draw up to `maxWithdraw`, and whichever executes
 * second finds the vault short, reverts its fill batch on chain and costs its
 * gas. That cost is accepted in exchange for having no local reservation that
 * could drift from the vault.
 *
 * Refreshes are serialised by the planner's per-chain mutex.
 */
export class VaultLiquidityState {
	private readonly logger: Logger

	/** Keyed by underlying asset address, lowercased. */
	private vaults = new Map<string, HydratedVault>()
	private hydrated = false

	constructor(
		private readonly chain: string,
		private readonly configs: VaultConfig[],
		private readonly solver: HexString,
		private readonly clientManager: ChainClientManager,
	) {
		this.logger = moduleLogger(clientManager.loggers, "vault-state")
	}

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
			let symbol = "TOKEN"
			try {
				symbol = (await client.readContract({
					address: asset,
					abi: ERC20_ABI,
					functionName: "symbol",
				})) as string
			} catch (err) {
				this.logger.warn({ err, chain: this.chain, asset }, "Could not resolve vault asset symbol")
			}

			// Vaults are keyed by underlying asset — a second same-asset vault would
			// silently shadow the first (its shares invisible to sourcing, sweeping
			// and shutdown redemption). Fail loudly instead.
			const existing = this.vaults.get(asset.toLowerCase())
			if (existing) {
				throw new Error(
					`Vaults ${existing.vault} and ${cfg.vault} on ${this.chain} share the underlying asset ${asset}; ` +
						"configure at most one vault per asset per chain",
				)
			}

			this.vaults.set(asset.toLowerCase(), {
				vault: cfg.vault,
				asset,
				symbol,
				decimals,
				thresholdScaled: cfg.threshold ? parseUnits(cfg.threshold, decimals) : null,
				minBalanceScaled: cfg.minBalance ? parseUnits(cfg.minBalance, decimals) : 0n,
				redeemOnShutdown: cfg.redeemOnShutdown ?? false,
				positionAssets: 0n,
				maxWithdrawable: 0n,
				maxDeposit: 0n,
			})
		}

		await this.refresh()
		this.hydrated = true

		for (const v of this.vaults.values()) {
			this.logger.info(
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
		this.logger.info({ chain: this.chain, vaults: this.configs.length }, "Vault liquidity state hydrated")
	}

	/**
	 * Refreshes live balances for every vault: the solver's position in asset
	 * terms, the vault's withdraw cap and its deposit cap.
	 */
	async refresh(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const v of this.vaults.values()) {
			const shares = (await client.readContract({
				address: v.vault,
				abi: ERC20_ABI,
				functionName: "balanceOf",
				args: [this.solver],
			})) as bigint

			const [positionAssets, maxWithdrawable, maxDeposit] = await Promise.all([
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
				client.readContract({
					address: v.vault,
					abi: ERC4626_ABI,
					functionName: "maxDeposit",
					args: [this.solver],
				}) as Promise<bigint>,
			])

			v.positionAssets = positionAssets
			v.maxWithdrawable = maxWithdrawable
			v.maxDeposit = maxDeposit

			this.logger.debug(
				{
					chain: this.chain,
					vault: v.vault,
					asset: v.asset,
					positionAssets: positionAssets.toString(),
					maxWithdrawable: maxWithdrawable.toString(),
					maxDeposit: maxDeposit.toString(),
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

	/** What the vault let the solver withdraw of `asset` at the last refresh: `maxWithdraw(solver)`. */
	withdrawable(asset: string): bigint {
		return this.vaults.get(asset.toLowerCase())?.maxWithdrawable ?? 0n
	}

	/** Wallet floor (`minBalance`) for `asset` the fill must never spend below. 0 if not configured. */
	reserveFor(asset: string): bigint {
		return this.vaults.get(asset.toLowerCase())?.minBalanceScaled ?? 0n
	}
}
