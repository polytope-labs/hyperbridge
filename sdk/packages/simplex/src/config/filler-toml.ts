import type { TunnelConfig } from "@/services/tunnel/TunnelService"
import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ConfirmationPolicy, DEFAULT_CONFIRMATION_POLICIES } from "@/config/interpolated-curve"
import { validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
import type { SignerConfig } from "@/services/wallet"
import { MIN_BLOCK_SCAN_INTERVAL_SECONDS } from "@/services/FillerConfigService"
import { MIN_ORDER_TTL_SECONDS } from "@/orderbook/types"
import type { UserProvidedChainConfig, AllowlistConfig } from "@/services/FillerConfigService"
import type { PaymasterKeeperConfig } from "@/services/PaymasterKeeperService"

export interface ChainConfirmationPolicy {
	/**
	 * Array of (amount, value) coordinates defining the confirmation curve.
	 * value = number of confirmations at that order amount
	 */
	points: Array<{
		amount: string
		value: number
	}>
}

/**
 * TOML row for an ERC-4626 vault entry. `threshold` (absolute human units) is the
 * high-water mark that triggers a sweep down to `minBalance`; omit both for
 * withdraw-only sourcing.
 */
export interface VaultToml {
	chain: string
	vault: HexString
	threshold?: string
	minBalance?: string
	redeemOnShutdown?: boolean
}

/** Top-level `[vault]` config: ERC-4626 treasury vaults, for withdraw sourcing and threshold sweeping. */
export interface VaultTomlConfig {
	vaults?: VaultToml[]
	sweepIntervalMs?: number
}

export interface QueueConfig {
	maxRechecks: number
	recheckDelayMs: number
}

export interface RebalancingConfig {
	triggerPercentage: number
	baseBalances: {
		USDC?: Record<string, string>
		USDT?: Record<string, string>
	}
}

export interface BinanceConfig {
	apiKey: string
	apiSecret: string
	basePath?: string
	timeout?: number
	depositTimeoutMs?: number
	pollIntervalMs?: number
	withdrawTimeoutMs?: number
}

export interface FillerTomlConfig {
	/**
	 * Optional asset-registry escape hatch: symbol → { chain → address }. Only
	 * needed for assets the built-in registry does not ship, or to override a
	 * shipped address for a deployment. Shipped symbols: USDC, USDT, DAI, CNGN
	 * (SDK chain registry) + curated USDR, ZARP, EURC, XSGD, TRYB and BRIDGE.
	 */
	assets?: Record<string, AssetDefinition>
	/**
	 * Per-chain confirmation policies for cross-chain orders, keyed by chain id.
	 * Merged over built-in defaults (ETH, BSC, Polygon, Base, Arbitrum,
	 * Unichain); every configured chain must be covered or startup fails. The
	 * policy's amount axis is the order's USD value: USD stables count at $1 and
	 * other tokens are valued through the open limit orders' rates. An order no
	 * limit order connects to a dollar waits for the policy's maximum depth.
	 */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
	simplex: {
		/** Orders evaluated at once. Defaults to 5. */
		maxConcurrentOrders?: number
		/**
		 * Accepted and ignored. Older configs and the wizard used to write it; the
		 * engine never read it. Kept in the type so those configs still load.
		 */
		queue?: QueueConfig
		logging?: string
		watchOnly?: boolean | Record<string, boolean>
		/**
		 * The Hyperbridge account that signs bid extrinsics and holds BRIDGE for their deposits.
		 * Optional: without it the solver bids on the HyperFX orderbook alone.
		 */
		substratePrivateKey?: string
		/** The Hyperbridge WebSocket endpoint bids go to. Required when a key is set. */
		hyperbridgeWsUrl?: string
		/**
		 * Hyperbridge indexer GraphQL endpoint, used to backfill order details on
		 * activity rows recorded before they were captured. Defaults per network
		 * (nexus for mainnet, gargantua for testnet).
		 */
		indexerUrl?: string
		/** Accepted and ignored. Contract addresses come from the SDK chain registry. */
		entryPointAddress?: string
		/** Accepted and ignored. Contract addresses come from the SDK chain registry. */
		solverAccountContractAddress?: string
		/** Target gas units for EntryPoint deposits per chain. Defaults to 3,000,000. */
		targetGasUnits?: number
		/**
		 * Block-scanner poll period per chain in seconds. Defaults to 3, and
		 * fractional values are allowed (0.5 = twice a second). Each tick costs
		 * one `eth_blockNumber` + one `eth_getLogs` per chain per endpoint, so
		 * raising this is the lever for fitting inside a rate-limited RPC's
		 * budget; the cost is seeing new orders that much later.
		 */
		blockScanIntervalSeconds?: number
		/** Gas fee bump (percentages added to base gasPrice). Defaults: priority=8%, max=10%. */
		gasFeeBump?: {
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
		}
		/**
		 * Removed. `[simplex.overfillProtection]` was dropped when the overfill clamp
		 * and the self-halt were taken out. It is no longer read: boot warns when a
		 * config still carries it, and `simplex init` drops it. Not rejected, because
		 * `simplex init` wrote the section uncommented for existing operators, and
		 * refusing it would crash-loop their Docker deployments. Typed `unknown` so
		 * whatever an old file holds still parses.
		 */
		overfillProtection?: unknown
		/**
		 * How long a signed bid stays executable, in seconds. Defaults to 300 (5 minutes).
		 *
		 * Written into `FillOptions.validUntil` and enforced by `fillOrder`, which reverts
		 * `FillExpired` past it. This is how long the quoted price stands as a firm commitment:
		 * the order's `deadline` is chosen by the placer with no ceiling, and retracting the bid
		 * on Hyperbridge does not reach the destination chain, so without it a signed bid stays
		 * executable indefinitely and is taken up only once the rate has moved against us.
		 *
		 * Configured in seconds but written on-chain in blocks, converted per destination chain.
		 */
		bidValiditySeconds?: number
		/**
		 * The order size, in USD, below which `order.fees` must cover the execution cost
		 * (fill gas plus, cross-chain, the relayer fee). Defaults to 20.
		 *
		 * An order worth at least this much is filled whatever fees it carries: the margin
		 * in the limit order that prices it pays for the gas. On a smaller order that
		 * margin is too thin to, so the fees have to. Set to 0 to never check fees. An
		 * order nothing can put a dollar figure on is treated as below it.
		 */
		minOrderSizeUsd?: number
		/**
		 * The smallest payout, in USD, a partial fill may bid. Defaults to 20.
		 *
		 * A partial fill collects none of the order's fees, and is deliberately exempt from
		 * the fee check and the profit floor: its gas is paid by the margin in the limit
		 * order that prices it. That leaves it with no size floor of its own, so a
		 * near-exhausted limit order, or a wallet just above its gas reserve, used to bid
		 * dust on every matching order — each bid paying fill gas and the relayer fee.
		 * Bids worth less than this are skipped. Set to 0 to bid whatever can be filled.
		 * Full fills are unaffected.
		 */
		minPartialFillUsd?: number
		/**
		 * Remote access: an outbound SSH tunnel to a rendezvous relay so a phone's
		 * SSH client can reach the local web UI. Off unless `enabled = true`; the
		 * relay defaults to the hosted one. Devices are paired from the UI.
		 */
		tunnel?: TunnelConfig
	}
	chains: UserProvidedChainConfig[]
	rebalancing?: RebalancingConfig
	binance?: BinanceConfig
	/** Filler-wide vault config: stablecoin sourcing for fills + threshold sweeping. */
	vault?: VaultTomlConfig
	/** Restricts order processing to listed user addresses. Omit to accept all users. */
	allowlist?: AllowlistConfig
	/** SimplexPaymaster fee-recycling keeper (`paymaster-keeper` subcommand). */
	keeper?: PaymasterKeeperConfig
	/** The HyperFX orderbook simplex posts its limit orders to. */
	orderbook?: OrderbookConfig
}

/**
 * Where the operator's limit orders are advertised.
 *
 * The limit orders themselves are not configured here: they live in `bids.db`
 * and are created over the API, because they are inventory the operator opens
 * and closes while the filler runs rather than startup settings.
 */
export interface OrderbookConfig {
	/** GraphQL endpoint. */
	url: string
	/**
	 * How long a limit order lives, in seconds, and the TTL written into its posting,
	 * when the create request names none. Defaults to 365 days; at least 900, the
	 * orderbook's floor. The order and its posting expire together: there is one
	 * clock, and nothing renews it.
	 */
	defaultTtlSecs?: number
	/** How often to reconcile local limit orders against the orderbook, in seconds. */
	reconcileIntervalSecs?: number
	requestTimeoutMs?: number
}

/**
 * The TOML file the binary reads: a {@link FillerTomlConfig} plus the
 * `[simplex.signer]` block, which is the CLI's way of naming a signing backend.
 *
 * The block is not part of the library's config — `Simplex.start` takes a
 * `Signer` instance — so it lives on this type and nowhere else. The binary
 * resolves it with `signerFromToml` and passes the parsed file straight through;
 * the extra key rides along untouched so the dashboard's config writer can put
 * it back in the file it came from.
 */
export interface FillerConfigFile extends FillerTomlConfig {
	simplex: FillerTomlConfig["simplex"] & {
		/** Omitted by watch-only configs, which sign nothing. */
		signer?: SignerConfig
	}
}

/**
 * Boot-parity confirmation coverage: every configured chain id needs a
 * confirmation curve (built-in defaults or [confirmationPolicies]), or orders
 * sourced on it would be silently dropped. Boot and both wizard write gates
 * run the same construction; the gates know the chain ids the wizard selected,
 * boot the ids resolved from the RPCs.
 */
export function assertConfirmationCoverage(
	confirmationPolicies: FillerTomlConfig["confirmationPolicies"],
	chainIds: number[],
): ConfirmationPolicy {
	const policy = new ConfirmationPolicy({
		...DEFAULT_CONFIRMATION_POLICIES,
		...(confirmationPolicies ?? {}),
	})
	policy.assertCovers(chainIds)
	return policy
}

/**
 * Validates raw TOML `[vault].vaults` entries. Pure and browser-safe — shared
 * by `validateConfig`, the vault-update endpoint and the funding planner.
 * Throws on missing/invalid required fields.
 */
export function validateVaultToml(
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

/**
 * The one check every way of setting rebalancing goes through: the TOML at boot, the library's
 * `RebalanceController.set`, and the UI's PUT. With no `triggerPercentage` rebalancing is switched
 * off (boot starts the service only when one is set), so base balances alone are inert and are not
 * checked. Otherwise the threshold is `base × (1 − p)`: a trigger of 0 fires whenever a balance is
 * at or below its base, 1 fires only at a zero balance, above 1 never fires, and NaN fails every
 * comparison, so each would run away with rebalancing or quietly disable it. A negative or
 * unparseable base would turn the threshold negative or NaN the same way. A base of `0` or `"0"` is
 * finite and not negative, so it passes (`FillerConfigService.getBaseBalance` skips a bare 0 as not
 * configured). Base balances are parsed with `parseFloat`, as `getBaseBalance` reads them.
 */
export function validateRebalancingConfig(rebalancing: RebalancingConfig): void {
	const trigger: unknown = rebalancing.triggerPercentage
	if (trigger === undefined) return
	if (typeof trigger !== "number" || !Number.isFinite(trigger) || trigger <= 0 || trigger >= 1) {
		throw new Error(
			`rebalancing.triggerPercentage must be a number between 0 and 1 (exclusive); got ${describeValue(trigger)}`,
		)
	}
	for (const asset of ["USDC", "USDT"] as const) {
		for (const [chainId, amount] of Object.entries(rebalancing.baseBalances?.[asset] ?? {}) as [string, unknown][]) {
			// A base balance is written as a string; parseFloat also takes a bare TOML number.
			const parsed =
				typeof amount === "string" || typeof amount === "number" ? Number.parseFloat(String(amount)) : Number.NaN
			if (!Number.isFinite(parsed) || parsed < 0) {
				throw new Error(
					`rebalancing.baseBalances.${asset} for chain ${chainId} must be a number >= 0; got ${describeValue(amount)}`,
				)
			}
		}
	}
}

/** A config value as an error message should show it: its type when it is not a number. */
function describeValue(value: unknown): string {
	if (typeof value === "number") return String(value)
	if (typeof value === "string") return `the string ${JSON.stringify(value)}`
	return value === null ? "null" : typeof value
}

/**
 * Checked at the gate rather than at first use: a misconfigured orderbook means
 * every limit order the operator creates is refused, and a
 * TTL under the orderbook's own floor is refused one order at a time with a
 * `TTL_TOO_SHORT` nobody sees until they try.
 */
function validateOrderbookConfig(orderbook: OrderbookConfig): void {
	if (!orderbook.url) {
		throw new Error("orderbook.url is required")
	}
	const positiveSeconds: [keyof OrderbookConfig, number | undefined, number][] = [
		["defaultTtlSecs", orderbook.defaultTtlSecs, MIN_ORDER_TTL_SECONDS],
		["reconcileIntervalSecs", orderbook.reconcileIntervalSecs, 1],
		["requestTimeoutMs", orderbook.requestTimeoutMs, 1],
	]
	for (const [name, value, minimum] of positiveSeconds) {
		if (value === undefined) continue
		if (!Number.isInteger(value) || value < minimum) {
			throw new Error(`orderbook.${name} must be an integer >= ${minimum}; got ${value}`)
		}
	}
}

export function validateConfig(config: FillerTomlConfig, cliWatchOnly = false): void {
	// The [[strategies]] array was removed when limit orders took over pricing.
	// Fail loudly so stale configs are migrated, not ignored.
	if ("strategies" in config) {
		throw new Error(
			"[[strategies]] was removed — simplex prices every fill from the limit orders you post. Run `simplex init` to migrate this config; per-strategy confirmation policies move to [confirmationPolicies]",
		)
	}

	// The --watch-only CLI flag forces global watch-only, so honour it here too
	// (otherwise the flag's own config would still trip the checks it exempts).
	const allChainsWatchOnly = cliWatchOnly || config.simplex?.watchOnly === true

	// A solver without a Hyperbridge key bids on the orderbook alone; with one it needs somewhere to send bids.
	if (config.simplex?.substratePrivateKey && !config.simplex.hyperbridgeWsUrl) {
		throw new Error("simplex.hyperbridgeWsUrl is required when simplex.substratePrivateKey is set")
	}

	if (!config.chains || config.chains.length === 0) {
		throw new Error("At least one chain must be configured")
	}

	// Validate chain configurations
	for (const chain of config.chains) {
		if (!Array.isArray(chain.rpcUrls) || chain.rpcUrls.length === 0 || chain.rpcUrls.some((u) => !u)) {
			throw new Error("Each chain configuration must have rpcUrls (a non-empty array of strings)")
		}
	}

	// `|| 5` downstream reads 0 as "unset", and p-queue throws a bare TypeError on
	// fractional or negative concurrency — surface both here with a named error.
	const concurrent = config.simplex.maxConcurrentOrders
	if (concurrent !== undefined) {
		if (!Number.isInteger(concurrent) || concurrent < 1) {
			throw new Error(`simplex.maxConcurrentOrders must be an integer >= 1; got ${concurrent}`)
		}
	}

	// A zero/negative/NaN interval would spin the scanner as fast as the event
	// loop allows and exhaust any RPC budget in minutes, so reject it at the gate
	// rather than letting setInterval coerce it.
	const scanInterval = config.simplex.blockScanIntervalSeconds
	if (scanInterval !== undefined) {
		if (!Number.isFinite(scanInterval) || scanInterval < MIN_BLOCK_SCAN_INTERVAL_SECONDS) {
			throw new Error(
				`simplex.blockScanIntervalSeconds must be a number >= ${MIN_BLOCK_SCAN_INTERVAL_SECONDS} (seconds); got ${scanInterval}`,
			)
		}
	}

	const minOrderSize = config.simplex.minOrderSizeUsd
	if (minOrderSize !== undefined) {
		if (!Number.isFinite(minOrderSize) || minOrderSize < 0) {
			throw new Error(`simplex.minOrderSizeUsd must be a number >= 0 (USD); got ${minOrderSize}`)
		}
	}

	const minPartialFill = config.simplex.minPartialFillUsd
	if (minPartialFill !== undefined) {
		if (!Number.isFinite(minPartialFill) || minPartialFill < 0) {
			throw new Error(`simplex.minPartialFillUsd must be a number >= 0 (USD); got ${minPartialFill}`)
		}
	}

	// Validate allowlist addresses (when present)
	if (config.allowlist) {
		for (const user of config.allowlist.users ?? []) {
			if (!isAddress(user)) {
				throw new Error(`allowlist.users contains an invalid address: ${user}`)
			}
		}
		for (const [chain, users] of Object.entries(config.allowlist.bySource ?? {})) {
			if (!Array.isArray(users)) {
				throw new Error(`allowlist.bySource."${chain}" must be an array of addresses`)
			}
			for (const user of users) {
				if (!isAddress(user)) {
					throw new Error(`allowlist.bySource."${chain}" contains an invalid address: ${user}`)
				}
			}
		}
	}

	if (config.vault?.vaults?.length) {
		validateVaultToml(config.vault.vaults)
	}

	if (config.rebalancing) {
		validateRebalancingConfig(config.rebalancing)
	}

	// Simplex prices from the operator's limit orders and those live on the
	// orderbook, so there is no configuration in which it is absent.
	if (!config.orderbook) {
		throw new Error("an [orderbook] section is required")
	}
	validateOrderbookConfig(config.orderbook)

	if (config.assets) {
		validateAssetDefinitions(config.assets)
	}

	// Per-chain confirmation policies (merged over built-in defaults at
	// startup). Constructing the real policy runs the exact boot-time
	// validation — chain-id keys, ≥ 2 points, non-negative integer values.
	if (config.confirmationPolicies) {
		void new ConfirmationPolicy(config.confirmationPolicies)
	}
}
