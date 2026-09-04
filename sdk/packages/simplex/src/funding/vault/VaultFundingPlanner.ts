import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { validateVaultToml } from "@/config/filler-toml"
import { VaultLiquidityState } from "@/funding/vault/VaultLiquidityState"
import type {
	FundingPlanResult,
	FundingVenue,
	HydratedVault,
	VaultBalancePosition,
	VaultOutputFundingConfig,
} from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { UserOpSender } from "@/services/UserOpSender"
import { type Logger, moduleLogger } from "@/services/Logger"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { Mutex } from "async-mutex"
import type { Decimal } from "decimal.js"
import { encodeFunctionData } from "viem"
/** Default sweep cadence when the config omits `sweepIntervalMs`. */
const DEFAULT_SWEEP_INTERVAL_MS = 5 * 60 * 1000

/**
 * Why a sweep pass left a configured vault alone. `deposits-closed` is the one an operator needs
 * to see: the wallet is over its trigger but the vault reports `maxDeposit(solver) == 0`.
 */
export type VaultSweepSkipReason = "sweeping-disabled" | "below-threshold" | "deposits-closed"

export interface VaultSweepSkip {
	chain: string
	vault: HexString
	asset: HexString
	symbol: string
	decimals: number
	reason: VaultSweepSkipReason
	/** Solver wallet balance of the underlying, base units. Not read when sweeping is disabled. */
	walletBalance?: bigint
	/** High-water trigger, base units. Absent when sweeping is disabled. */
	threshold?: bigint
	/** `maxDeposit(solver)` at the time of the pass. Only read once the threshold is met. */
	maxDeposit?: bigint
}

export interface VaultSweepDeposit {
	vault: HexString
	asset: HexString
	symbol: string
	decimals: number
	/** Amount deposited, base units. */
	amount: bigint
}

export interface VaultSweepSubmission {
	chain: string
	txHash: HexString
	sponsored: boolean
	deposits: VaultSweepDeposit[]
}

/** Outcome of one sweep pass: what was submitted, and why every other vault was left alone. */
export interface VaultSweepResult {
	submitted: VaultSweepSubmission[]
	skipped: VaultSweepSkip[]
}

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
	private readonly logger: Logger

	name = "Vault"
	private stateByChain = new Map<string, VaultLiquidityState>()
	private mutexByChain = new Map<string, Mutex>()
	/** Per-chain mutex serialising sweeps so a slow supply tx can't overlap the next tick. */
	private sweepMutexByChain = new Map<string, Mutex>()
	private solver: HexString | null = null
	private sweepInterval?: NodeJS.Timeout
	private initialSweepTimer?: NodeJS.Timeout
	/**
	 * `chain:vault` keys whose closed-deposit state has been warned about. The periodic sweep runs
	 * every few minutes and a streaming-yield vault is closed for most of the day; one warn per
	 * closure (and debug thereafter) keeps the log readable without hiding the cause.
	 */
	private depositsClosedWarned = new Set<string>()

	/**
	 * @param userOpSender When provided, sweep/redeem batches are sent as Circle-
	 * Paymaster-sponsored UserOps (gas paid in USDC) where the chain supports it,
	 * falling back to a native EOA tx. Omit to always use native txs.
	 */
	constructor(
		private readonly clientManager: ChainClientManager,
		private config: VaultOutputFundingConfig,
		private readonly userOpSender?: UserOpSender,
	) {
		this.logger = moduleLogger(clientManager.loggers, "vault-funding")
	}

	/** Invoked after each submitted sweep/redeem batch so wallet history can record it. */
	onTx?: (tx: { chain: string; kind: "sweep" | "redeem"; txHash: HexString; sponsored: boolean }) => void

	/**
	 * Replaces the vault set at runtime and re-hydrates. The instance is shared
	 * with every strategy's funding-venue list, so an in-place swap takes effect
	 * on the next fill/sweep without re-wiring. Sweeping restarts if it was on.
	 */
	async reconfigure(config: VaultOutputFundingConfig): Promise<void> {
		const solver = this.solver
		if (!solver) throw new Error("Vault venue is not initialised yet")

		const wasSweeping = this.sweepInterval !== undefined
		this.stopSweeping()

		const stateByChain = new Map<string, VaultLiquidityState>()
		for (const [chain, vaults] of Object.entries(config.vaultsByChain)) {
			const state = new VaultLiquidityState(chain, vaults, solver, this.clientManager)
			await state.hydrate()
			stateByChain.set(chain, state)
		}

		// Swap only after every chain hydrated, so a bad address leaves the old set live.
		this.config = config
		this.stateByChain.clear()
		this.depositsClosedWarned.clear()
		for (const [chain, state] of stateByChain) {
			this.stateByChain.set(chain, state)
			if (!this.mutexByChain.has(chain)) this.mutexByChain.set(chain, new Mutex())
			if (!this.sweepMutexByChain.has(chain)) this.sweepMutexByChain.set(chain, new Mutex())
		}

		if (wasSweeping) this.startSweeping()
		this.logger.info({ chains: Object.keys(config.vaultsByChain) }, "Vault venue reconfigured")
	}

	/**
	 * Validates raw TOML vault entries before constructing the planner.
	 * Throws on missing/invalid required fields.
	 */
	static validateConfig(
		vaults: { chain?: string; vault?: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }[],
	): void {
		validateVaultToml(vaults)
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
			this.logger.info({ chain, vaultCount: vaults.length, solver }, "Vault venue initialising chain")

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

	/**
	 * Returns a coherent read-only view of every configured vault. Refreshing
	 * under the same per-chain mutex used by withdrawal planning ensures the
	 * displayed availability includes all live pending-fill reservations.
	 */
	async getBalanceSnapshot(requestedChain?: string): Promise<VaultBalancePosition[]> {
		const snapshots = await Promise.all(
			Array.from(this.stateByChain.entries())
				.filter(([chain]) => requestedChain === undefined || chain === requestedChain)
				.map(async ([chain, state]) => {
				const mutex = this.mutexByChain.get(chain)
				if (!mutex || !state.isHydrated()) return []

				return mutex.runExclusive(async () => {
					await state.refresh()
					return state.allVaults().map((vault) => ({
						chain,
						vault: vault.vault,
						asset: vault.asset,
						symbol: vault.symbol,
						decimals: vault.decimals,
						positionAssets: vault.positionAssets,
						availableAssets: vault.remaining,
						walletReserve: vault.minBalanceScaled,
						acceptsDeposits: vault.maxDeposit > 0n,
					}))
				})
				}),
		)

		return snapshots.flat()
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

			this.logger.debug(
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
		this.initialSweepTimer = setTimeout(() => {
			this.sweepExcessToVault().catch((err) => this.logger.error({ err }, "Vault initial sweep failed"))
		}, 30_000)

		this.sweepInterval = setInterval(() => {
			this.sweepExcessToVault().catch((err) => this.logger.error({ err }, "Vault periodic sweep failed"))
		}, intervalMs)

		this.logger.info({ intervalMs }, "Vault periodic sweep started")
	}

	stopSweeping(): void {
		// Tracked as well as the interval: the first sweep is a one-shot timer, and
		// an untracked one fires after stop() has already resolved.
		if (this.initialSweepTimer) {
			clearTimeout(this.initialSweepTimer)
			this.initialSweepTimer = undefined
		}
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
	async sweepExcessToVault(chain?: string): Promise<VaultSweepResult> {
		const chains = chain ? [chain] : Array.from(this.stateByChain.keys())
		const passes = await Promise.all(chains.map((c) => this.sweepChain(c)))
		return {
			submitted: passes.flatMap((pass) => (pass.submitted ? [pass.submitted] : [])),
			skipped: passes.flatMap((pass) => pass.skipped),
		}
	}

	private async sweepChain(chain: string): Promise<{ submitted?: VaultSweepSubmission; skipped: VaultSweepSkip[] }> {
		const state = this.stateByChain.get(chain)
		const solver = this.solver
		if (!state || !state.isHydrated() || !solver) return { skipped: [] }

		const mutex = this.sweepMutexByChain.get(chain)!
		return mutex.runExclusive(async () => {
			const publicClient = this.clientManager.getPublicClient(chain)
			const calls: ERC7821Call[] = []
			const deposits: VaultSweepDeposit[] = []
			const skipped: VaultSweepSkip[] = []

			for (const vault of state.allVaults()) {
				const identity = { chain, vault: vault.vault, asset: vault.asset, symbol: vault.symbol, decimals: vault.decimals }
				if (vault.thresholdScaled === null) {
					skipped.push({ ...identity, reason: "sweeping-disabled" })
					continue
				}

				const walletBalance = (await publicClient.readContract({
					abi: ERC20_ABI,
					address: vault.asset,
					functionName: "balanceOf",
					args: [solver],
				})) as bigint

				// Hysteresis: only act once the balance reaches the high-water
				// trigger, then deposit everything down to minBalance. The
				// threshold→minBalance gap is the implicit minimum sweep size.
				if (walletBalance < vault.thresholdScaled) {
					skipped.push({ ...identity, reason: "below-threshold", walletBalance, threshold: vault.thresholdScaled })
					continue
				}
				const excess = walletBalance - vault.minBalanceScaled

				// Clamp to the vault's deposit cap — ERC-4626 requires deposit to
				// revert when assets > maxDeposit(receiver) (e.g. an Aave stataToken
				// at its supply cap, a paused market, or a StreamingYieldVault while a
				// tranche vests). Without this the sweep tx reverts every tick once
				// the cap is hit. `excess` is read here and deposited in a later tx;
				// if a fill consumes wallet balance in the interim the batch reverts
				// atomically, leaving no stale allowance.
				const maxDeposit = (await publicClient.readContract({
					abi: ERC4626_ABI,
					address: vault.vault,
					functionName: "maxDeposit",
					args: [solver],
				})) as bigint
				const depositAmount = excess < maxDeposit ? excess : maxDeposit
				if (depositAmount <= 0n) {
					// The wallet is over its trigger and the vault refuses the deposit.
					// Silently skipping here made a closed vault indistinguishable from
					// a wallet that never reached its threshold.
					this.noteDepositsClosed(chain, vault, walletBalance, maxDeposit)
					skipped.push({
						...identity,
						reason: "deposits-closed",
						walletBalance,
						threshold: vault.thresholdScaled,
						maxDeposit,
					})
					continue
				}
				this.depositsClosedWarned.delete(closedKey(chain, vault))

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
				deposits.push({ ...identity, amount: depositAmount })

				this.logger.info(
					{ chain, vault: vault.vault, asset: vault.asset, excess: excess.toString(), depositAmount: depositAmount.toString() },
					"Vault sweeping excess in",
				)
			}

			if (calls.length === 0) return { skipped }

			const { txHash, sponsored } = await this.submitBatch(chain, solver, calls)
			this.logger.info({ chain, tx: txHash, sponsored, pairs: calls.length / 2 }, "Vault sweep submitted")
			this.onTx?.({ chain, kind: "sweep", txHash, sponsored })
			return { submitted: { chain, txHash, sponsored, deposits }, skipped }
		})
	}

	/**
	 * Warns the first time a vault turns a due sweep away, debug on every repeat until a deposit
	 * goes through (or the venue is reconfigured). The fields name what an operator needs to
	 * decide whether to wait or intervene: how far over the trigger the wallet is, and that the
	 * vault — not the solver — is the side saying no.
	 */
	private noteDepositsClosed(chain: string, vault: HydratedVault, walletBalance: bigint, maxDeposit: bigint): void {
		const key = closedKey(chain, vault)
		const fields = {
			chain,
			vault: vault.vault,
			asset: vault.asset,
			symbol: vault.symbol,
			walletBalance: walletBalance.toString(),
			threshold: vault.thresholdScaled?.toString(),
			minBalance: vault.minBalanceScaled.toString(),
			maxDeposit: maxDeposit.toString(),
		}
		if (this.depositsClosedWarned.has(key)) {
			this.logger.debug(fields, "Vault sweep skipped: vault still not accepting deposits")
			return
		}
		this.depositsClosedWarned.add(key)
		this.logger.warn(
			fields,
			"Vault sweep skipped: wallet balance is above its sweep threshold but the vault is not accepting deposits (maxDeposit is 0); retrying every cycle",
		)
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
			// The bundler echoes input gas limits for these ops instead of simulating, so
			// pass measured fixed limits. Verification efficiency `used / (verif + pmVerif)`
			// must clear rundler's 0.4 floor — the Circle paymaster verification (~75k) is
			// the dominant term, account validation only ~24k.
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData,
				gas: {
					verificationGasLimit: 60_000n,
					callGasLimit: 350_000n * BigInt(calls.length) + 100_000n,
					preVerificationGas: 150_000n,
				},
				paymasterVerificationGasLimit: 140_000n,
			})
			if (result) return { txHash: result.txHash, sponsored: true }
			this.logger.warn({ chain }, "Sponsored batch unavailable, sending native tx")
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
				this.redeemChain(c).catch((err) => this.logger.error({ err, chain: c }, "Vault shutdown redeem failed")),
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

				this.logger.info(
					{ chain, vault: vault.vault, asset: vault.asset, shares: shares.toString() },
					"Vault redeeming full position",
				)
			}

			if (calls.length === 0) return

			const { txHash, sponsored } = await this.submitBatch(chain, solver, calls)
			this.logger.info({ chain, tx: txHash, sponsored, vaults: calls.length }, "Vault shutdown redeem submitted")
			this.onTx?.({ chain, kind: "redeem", txHash, sponsored })
		})
	}
}

function closedKey(chain: string, vault: HydratedVault): string {
	return `${chain}:${vault.vault.toLowerCase()}`
}
