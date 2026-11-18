import type { TunnelConfig } from "@/services/tunnel/TunnelService"
import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ConfirmationPolicy, DEFAULT_CONFIRMATION_POLICIES } from "@/config/interpolated-curve"
import { normalizeSymbol, validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
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

/**
 * TOML row for a `[[rebalancing.hubs]]` entry: the chain one asset is gathered on. Once any
 * other chain's wallet holds `threshold` of the asset, everything above `minBalance` is
 * bridged to `chain`. Amounts are in human units.
 */
export interface RebalancingHubToml {
	/** Asset symbol, resolved through the asset registry (e.g. "USDC"). */
	asset: string
	/** The hub chain's state machine id (e.g. "EVM-8453"). */
	chain: string
	threshold: string
	minBalance: string
}

/** Top-level `[rebalancing]` config: one hub chain per asset. */
export interface RebalancingConfig {
	hubs?: RebalancingHubToml[]
	/** How often wallet balances are checked, in milliseconds. Defaults to 5 minutes. */
	intervalMs?: number
}

/** Rebalancing check cadence when the config omits `intervalMs`. */
export const DEFAULT_REBALANCE_INTERVAL_MS = 5 * 60 * 1000

/** The shortest `rebalancing.intervalMs`: each check reads every chain's balance over RPC. */
export const MIN_REBALANCE_INTERVAL_MS = 10_000

/** The longest: a timer armed with a longer delay fires after one millisecond instead. */
export const MAX_REBALANCE_INTERVAL_MS = 2_147_483_647

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
	/**
	 * Whether heartbeats are signed by a key held in memory that the signer authorises
	 * every couple of hours, rather than by the signer itself every tick. Defaults to true.
	 */
	delegateHeartbeats?: boolean
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

/** The settings rebalancing had before hubs. */
const REMOVED_REBALANCING_KEYS = ["triggerPercentage", "baseBalances"]

/**
 * Whether a `[rebalancing]` section still carries the settings from before hubs. They cannot
 * be translated: a base balance per chain does not say which chain the funds should gather on.
 */
export function hasRemovedRebalancingKeys(rebalancing: unknown): boolean {
	return (
		typeof rebalancing === "object" &&
		rebalancing !== null &&
		REMOVED_REBALANCING_KEYS.some((key) => key in rebalancing)
	)
}

const REBALANCING_EXAMPLE = [
	"[[rebalancing.hubs]]",
	'asset      = "USDC"',
	'chain      = "EVM-8453"',
	'threshold  = "6000"',
	'minBalance = "5000"',
].join("\n")

/**
 * The one check every way of setting rebalancing goes through: the TOML at boot, the library's
 * `RebalanceController.set`, and the UI's PUT. Pure and browser-safe.
 *
 * Pass `chainIds`, the chains the solver runs, to also require every hub to be one of them.
 * `validateConfig` cannot: chain ids are only known once the RPCs have answered.
 */
export function validateRebalancingConfig(rebalancing: RebalancingConfig, chainIds?: number[]): void {
	if (typeof rebalancing !== "object" || rebalancing === null) {
		throw new Error(`rebalancing must be a [rebalancing] section; got ${describeValue(rebalancing)}`)
	}
	if (hasRemovedRebalancingKeys(rebalancing)) {
		const removed = REMOVED_REBALANCING_KEYS.filter((key) => key in rebalancing)
		throw new Error(
			`rebalancing.${removed.join(" and rebalancing.")} ${removed.length === 1 ? "was" : "were"} removed. ` +
				"Rebalancing now gathers each asset on a hub chain: once another chain holds `threshold`, " +
				`everything above \`minBalance\` is bridged to the hub. Replace the section with:\n\n${REBALANCING_EXAMPLE}\n\n` +
				"`simplex init` drops the old section when you choose to update the config.",
		)
	}

	const interval: unknown = rebalancing.intervalMs
	if (
		interval !== undefined &&
		(!Number.isInteger(interval) ||
			(interval as number) < MIN_REBALANCE_INTERVAL_MS ||
			(interval as number) > MAX_REBALANCE_INTERVAL_MS)
	) {
		throw new Error(
			`rebalancing.intervalMs must be an integer from ${MIN_REBALANCE_INTERVAL_MS} to ${MAX_REBALANCE_INTERVAL_MS} (milliseconds); got ${describeValue(interval)}`,
		)
	}

	const hubs: unknown = rebalancing.hubs ?? []
	if (!Array.isArray(hubs)) {
		throw new Error(`rebalancing.hubs must be a list of [[rebalancing.hubs]] entries; got ${describeValue(hubs)}`)
	}
	// Whole cents at most, as transfers are sent: a finer amount would let `threshold` and
	// `minBalance` differ on paper and scale to the same balance. Positive, like a vault's.
	const amount = (asset: string, field: "threshold" | "minBalance", value: unknown): number => {
		if (typeof value !== "string" || !/^\d+(\.\d{1,2})?$/.test(value) || Number(value) <= 0) {
			throw new Error(
				`rebalancing.hubs: ${asset} '${field}' must be a quoted positive amount with at most two decimals, such as "5000"; got ${describeValue(value)}`,
			)
		}
		return Number(value)
	}
	const seen = new Set<string>()
	for (const hub of hubs as Partial<RebalancingHubToml>[]) {
		if (typeof hub?.asset !== "string" || !hub.asset.trim()) {
			throw new Error("rebalancing.hubs: each hub must name an 'asset' (e.g. USDC)")
		}
		const asset = normalizeSymbol(hub.asset)
		if (seen.has(asset)) {
			throw new Error(`rebalancing.hubs: ${asset} has more than one hub; an asset gathers on one chain`)
		}
		seen.add(asset)

		// Exactly the id the solver names its chains by: "EVM-08453" would pass for a chain it
		// runs and then never equal it.
		if (typeof hub.chain !== "string" || !/^EVM-[1-9]\d*$/.test(hub.chain)) {
			throw new Error(
				`rebalancing.hubs: ${asset} 'chain' must be "EVM-<chainId>" (e.g. "EVM-8453"); got ${describeValue(hub.chain)}`,
			)
		}
		if (chainIds && !chainIds.includes(Number(hub.chain.slice("EVM-".length)))) {
			throw new Error(`rebalancing.hubs: ${asset}'s hub ${hub.chain} is not a chain this solver runs`)
		}

		const threshold = amount(asset, "threshold", hub.threshold)
		const minBalance = amount(asset, "minBalance", hub.minBalance)
		// The gap between the two is the smallest transfer, so it has to exist.
		if (threshold <= minBalance) {
			throw new Error(`rebalancing.hubs: ${asset} 'threshold' must be greater than 'minBalance'`)
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
	// A quoted "false" would otherwise leave delegation on without a word.
	if (orderbook.delegateHeartbeats !== undefined && typeof orderbook.delegateHeartbeats !== "boolean") {
		throw new Error(
			`orderbook.delegateHeartbeats must be true or false; got ${describeValue(orderbook.delegateHeartbeats)}`,
		)
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
