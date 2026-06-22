import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { VaultLiquidityState } from "@/funding/vault/VaultLiquidityState"
import type { VaultOutputFundingConfig, FundingPlanResult, FundingVenue } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { UserOpSender } from "@/services/UserOpSender"
import { getLogger } from "@/services/Logger"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { Mutex } from "async-mutex"
import type { Decimal } from "decimal.js"
import { encodeFunctionData } from "viem"

const logger = getLogger("vault-funding")

/** Default sweep cadence when the config omits `sweepIntervalMs`. */
const DEFAULT_SWEEP_INTERVAL_MS = 5 * 60 * 1000

/**
 * Funding venue that sources output tokens by withdrawing the solver's own
 * position from any ERC-4626 vault (`vault.withdraw`). The first concrete venue
 * is Aave's stataToken; any compliant vault (Morpho, an issuer yield vault)
 * works with a config entry and no code changes.
 *
 * Sourcing is one-sided per token — tokens not backed by a configured vault
 * yield a no-op plan so the caller falls back to the wallet balance or another
 * venue.
 *
 * Configured vaults hold stablecoins (USDC/USDT), so this venue does not price
 * exotic tokens: {@link getExoticTokenPrice} always returns null.
 */
export class VaultFundingPlanner implements FundingVenue {
	name = "Vault"
	private stateByChain = new Map<string, VaultLiquidityState>()
	private mutexByChain = new Map<string, Mutex>()
	/** Per-chain mutex serialising sweeps so a slow supply tx can't overlap the next tick. */
	private sweepMutexByChain = new Map<string, Mutex>()
	private solver: HexString | null = null
	private sweepInterval?: NodeJS.Timeout

	/**
	 * @param userOpSender When provided, sweep/redeem batches are sent as Circle-
	 * Paymaster-sponsored UserOps (gas paid in USDC) where the chain supports it,
	 * falling back to a native EOA tx. Omit to always use native txs.
	 */
	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: VaultOutputFundingConfig,
		private readonly userOpSender?: UserOpSender,
	) {}

	/**
	 * Validates raw TOML vault entries before constructing the planner.
	 * Throws on missing/invalid required fields.
	 */
	static validateConfig(
		vaults: { chain?: string; vault?: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }[],
	): void {
		const positiveNumber = (v: string) => /^\d+(\.\d+)?$/.test(v.trim()) && Number(v) > 0
		for (const v of vaults) {
			if (!v.chain?.trim()) {
				throw new Error("Each vault must have a non-empty 'chain' (e.g. EVM-8453)")
			}
			if (!v.vault?.trim()) {
				throw new Error("Each vault entry must include a 'vault' address")
			}
			if (v.threshold !== undefined && !positiveNumber(v.threshold)) {
				throw new Error(`Vault ${v.vault} 'threshold' must be a positive number`)
			}
			if (v.minBalance !== undefined && !positiveNumber(v.minBalance)) {
				throw new Error(`Vault ${v.vault} 'minBalance' must be a positive number`)
			}
			// Sweeping needs a floor to keep gas/paymaster funds, and a trigger
			// strictly above it so a sweep never tries to deposit ≤ 0.
			if (v.threshold !== undefined) {
				if (v.minBalance === undefined) {
					throw new Error(`Vault ${v.vault} sets 'threshold' so it must also set 'minBalance'`)
				}
				if (Number(v.threshold) <= Number(v.minBalance)) {
					throw new Error(`Vault ${v.vault} 'threshold' must be greater than 'minBalance'`)
				}
			}
		}
	}

	// =========================================================================
	// Lifecycle (FundingVenue)
	// =========================================================================

	async initialise(solver: HexString): Promise<void> {
		// Idempotent: the same shared instance is passed to multiple strategies,
		// each of which calls initialise() during its own startup.
		if (this.solver) return
		this.solver = solver
		for (const [chain, vaults] of Object.entries(this.config.vaultsByChain)) {
			logger.info({ chain, vaultCount: vaults.length, solver }, "Vault venue initialising chain")

			const state = new VaultLiquidityState(chain, vaults, solver, this.clientManager)
			await state.hydrate()
			this.stateByChain.set(chain, state)
			this.mutexByChain.set(chain, new Mutex())
			this.sweepMutexByChain.set(chain, new Mutex())
		}
	}

	async refresh(chain?: string): Promise<void> {
		if (chain) {
			const state = this.stateByChain.get(chain)
			if (state) await state.refresh()
		} else {
			await Promise.all(Array.from(this.stateByChain.values()).map((s) => s.refresh()))
		}
	}

	// =========================================================================
	// Pricing (FundingVenue)
	// =========================================================================

	/** Configured vaults hold stablecoins; this venue does not price exotic tokens. */
	async getExoticTokenPrice(_chain: string, _exoticToken: string): Promise<Decimal | null> {
		return null
	}

	/**
	 * The vault's `minBalance` floor for `tokenLower` on `chain` — the wallet
	 * balance the fill must keep liquid (gas/paymaster). 0 when no configured
	 * vault on the chain holds the token.
	 */
	walletReserveForToken(chain: string, tokenLower: string): bigint {
		const state = this.stateByChain.get(chain)
		if (!state || !state.isHydrated()) return 0n
		return state.reserveFor(tokenLower)
	}

	// =========================================================================
	// Planning (FundingVenue)
	// =========================================================================

	/**
	 * Produces a single `vault.withdraw` ERC-7821 call that sends up to
	 * `amountNeeded` of `tokenOutLower` to the solver, capped by the vault's
	 * `maxWithdraw`. Returns a no-op when no configured vault holds the token.
	 */
	async planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
		_deadlineTimestamp?: bigint,
	): Promise<FundingPlanResult> {
		const noopResult: FundingPlanResult = { calls: [], credited: 0n }

		if (amountNeeded <= 0n) return noopResult

		const state = this.stateByChain.get(destChain)
		if (!state || !state.isHydrated()) return noopResult

		const mutex = this.mutexByChain.get(destChain)!
		return mutex.runExclusive(async () => {
			await state.refresh()

			const tokenNeed = tokenOutLower.toLowerCase()
			const vault = state.vaultForToken(tokenNeed)
			if (!vault) return noopResult

			const available = state.remaining(vault.asset)
			if (available <= 0n) return noopResult

			const amount = amountNeeded < available ? amountNeeded : available

			const call: ERC7821Call = {
				target: vault.vault,
				value: 0n,
				data: encodeFunctionData({
					abi: ERC4626_ABI,
					functionName: "withdraw",
					args: [amount, solver, solver],
				}) as HexString,
			}

			state.consume(vault.asset, amount)

			logger.debug(
				{
					destChain,
					vault: vault.vault,
					asset: vault.asset,
					amountNeeded: amountNeeded.toString(),
					available: available.toString(),
					credited: amount.toString(),
				},
				"Vault funding planned",
			)

			return { calls: [call], credited: amount }
		})
	}

	// =========================================================================
	// Sweeping — deposit idle wallet balance into the vault
	// =========================================================================

	/**
	 * Starts the periodic sweep timer. Runs one sweep shortly after start, then
	 * every `sweepIntervalMs`. Idempotent — a second call is a no-op.
	 */
	startSweeping(): void {
		if (this.sweepInterval) return
		const intervalMs = this.config.sweepIntervalMs ?? DEFAULT_SWEEP_INTERVAL_MS

		// Initial sweep shortly after start (lets the filler settle first).
		setTimeout(() => {
			this.sweepExcessToVault().catch((err) => logger.error({ err }, "Vault initial sweep failed"))
		}, 30_000)

		this.sweepInterval = setInterval(() => {
			this.sweepExcessToVault().catch((err) => logger.error({ err }, "Vault periodic sweep failed"))
		}, intervalMs)

		logger.info({ intervalMs }, "Vault periodic sweep started")
	}

	stopSweeping(): void {
		if (this.sweepInterval) {
			clearInterval(this.sweepInterval)
			this.sweepInterval = undefined
		}
	}

	/**
	 * Deposits idle wallet balance into the vault for one chain (or all configured
	 * chains). For each vault whose wallet balance has reached its `threshold`
	 * high-water mark, deposits everything down to `minBalance`, building an exact
	 * `approve + deposit` pair and sending them as a single ERC-7821 batch to the
	 * solver account — atomic, leaving no residual allowance.
	 */
	async sweepExcessToVault(chain?: string): Promise<void> {
		const chains = chain ? [chain] : Array.from(this.stateByChain.keys())
		await Promise.all(chains.map((c) => this.sweepChain(c)))
	}

	private async sweepChain(chain: string): Promise<void> {
		const state = this.stateByChain.get(chain)
		const solver = this.solver
		if (!state || !state.isHydrated() || !solver) return

		const mutex = this.sweepMutexByChain.get(chain)!
		await mutex.runExclusive(async () => {
			const publicClient = this.clientManager.getPublicClient(chain)
			const calls: ERC7821Call[] = []

			for (const vault of state.allVaults()) {
				if (vault.thresholdScaled === null) continue // sweeping disabled for this vault

				const walletBalance = (await publicClient.readContract({
					abi: ERC20_ABI,
					address: vault.asset,
					functionName: "balanceOf",
					args: [solver],
				})) as bigint

				// Hysteresis: only act once the balance reaches the high-water
				// trigger, then deposit everything down to minBalance. The
				// threshold→minBalance gap is the implicit minimum sweep size.
				if (walletBalance < vault.thresholdScaled) continue
				const excess = walletBalance - vault.minBalanceScaled

				// Clamp to the vault's deposit cap — ERC-4626 requires deposit to
				// revert when assets > maxDeposit(receiver) (e.g. an Aave stataToken
				// at its supply cap, or a paused market). Without this the sweep tx
				// reverts every tick once the cap is hit. `excess` is read here and
				// deposited in a later tx; if a fill consumes wallet balance in the
				// interim the batch reverts atomically, leaving no stale allowance.
				const maxDeposit = (await publicClient.readContract({
					abi: ERC4626_ABI,
					address: vault.vault,
					functionName: "maxDeposit",
					args: [solver],
				})) as bigint
				const depositAmount = excess < maxDeposit ? excess : maxDeposit
				if (depositAmount <= 0n) continue

				calls.push({
					target: vault.asset,
					value: 0n,
					data: encodeFunctionData({
						abi: ERC20_ABI,
						functionName: "approve",
						args: [vault.vault, depositAmount],
					}) as HexString,
				})
				calls.push({
					target: vault.vault,
					value: 0n,
					data: encodeFunctionData({
						abi: ERC4626_ABI,
						functionName: "deposit",
						args: [depositAmount, solver],
					}) as HexString,
				})

				logger.info(
					{ chain, vault: vault.vault, asset: vault.asset, excess: excess.toString(), depositAmount: depositAmount.toString() },
					"Vault sweeping excess in",
				)
			}

			if (calls.length === 0) return

			const { txHash, sponsored } = await this.submitBatch(chain, solver, calls)
			logger.info({ chain, tx: txHash, sponsored, pairs: calls.length / 2 }, "Vault sweep submitted")
		})
	}

	/**
	 * Sends an ERC-7821 batch to the solver account. Prefers a Circle-Paymaster-
	 * sponsored UserOp (gas paid in USDC) when a sender is wired and the chain
	 * supports it, falling back to a native EOA tx.
	 *
	 * The sponsored path only falls back to native when the op was **never
	 * submitted**; a submitted-but-unconfirmed op throws so a native resend can't
	 * double-execute the batch (the caller's timer logs and retries next cycle).
	 */
	private async submitBatch(
		chain: string,
		solver: HexString,
		calls: ERC7821Call[],
	): Promise<{ txHash: HexString; sponsored: boolean }> {
		const callData = encodeERC7821ExecuteBatch(calls)

		if (this.userOpSender?.canSponsor(chain)) {
			const result = await this.userOpSender.trySendSponsored({ chain, callData })
			if (result) return { txHash: result.txHash, sponsored: true }
			logger.warn({ chain }, "Sponsored batch unavailable, sending native tx")
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)
		const tx = await walletClient.sendTransaction({
			to: solver,
			data: callData,
			value: 0n,
			chain: walletClient.chain,
		})
		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1, timeout: 60_000 })
		if (receipt.status !== "success") {
			throw new Error(`Vault batch tx reverted: ${tx}`)
		}
		return { txHash: tx, sponsored: false }
	}

	// =========================================================================
	// Shutdown — exit all vault positions back to the underlying asset
	// =========================================================================

	/**
	 * Redeems the solver's full share balance from every configured vault back
	 * into the underlying asset, one ERC-7821 batch per chain. Share-denominated
	 * `redeem` so no rounding dust is stranded. Per-chain failures are logged,
	 * not thrown — shutdown must not hang on one bad RPC.
	 */
	async redeemAll(): Promise<void> {
		const chains = Array.from(this.stateByChain.keys())
		await Promise.all(
			chains.map((c) =>
				this.redeemChain(c).catch((err) => logger.error({ err, chain: c }, "Vault shutdown redeem failed")),
			),
		)
	}

	private async redeemChain(chain: string): Promise<void> {
		const state = this.stateByChain.get(chain)
		const solver = this.solver
		if (!state || !state.isHydrated() || !solver) return

		const mutex = this.sweepMutexByChain.get(chain)!
		await mutex.runExclusive(async () => {
			const publicClient = this.clientManager.getPublicClient(chain)
			const calls: ERC7821Call[] = []

			for (const vault of state.allVaults()) {
				if (!vault.redeemOnShutdown) continue // operator opted to keep this position

				const shares = (await publicClient.readContract({
					abi: ERC20_ABI,
					address: vault.vault,
					functionName: "balanceOf",
					args: [solver],
				})) as bigint
				if (shares === 0n) continue

				calls.push({
					target: vault.vault,
					value: 0n,
					data: encodeFunctionData({
						abi: ERC4626_ABI,
						functionName: "redeem",
						args: [shares, solver, solver],
					}) as HexString,
				})

				logger.info(
					{ chain, vault: vault.vault, asset: vault.asset, shares: shares.toString() },
					"Vault redeeming full position",
				)
			}

			if (calls.length === 0) return

			const { txHash, sponsored } = await this.submitBatch(chain, solver, calls)
			logger.info({ chain, tx: txHash, sponsored, vaults: calls.length }, "Vault shutdown redeem submitted")
		})
	}
}
