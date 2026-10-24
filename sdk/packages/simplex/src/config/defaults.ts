/** Orders the engine evaluates concurrently when the config does not say. */
export const DEFAULT_MAX_CONCURRENT_ORDERS = 5

/** How long an orderbook request waits before it is treated as unreachable. */
export const DEFAULT_ORDERBOOK_TIMEOUT_MS = 10_000

/** The orderbook simplex posts to when the config does not name another, per network. */
export const DEFAULT_ORDERBOOK_URLS = {
	mainnet: "https://orderbook.hyperfx.finance/mainnet/graphql",
	testnet: "https://orderbook.hyperfx.finance/testnet/graphql",
} as const

/** Defaults earlier releases wrote, which no longer serve an orderbook. The wizard replaces them. */
export const RETIRED_ORDERBOOK_URLS: readonly string[] = ["https://orderbook.hyperbridge.network/graphql"]

/** How often to reconcile local limit orders against the orderbook's copy. */
export const DEFAULT_RECONCILE_INTERVAL_SECS = 300

/**
 * Whole tokens a limit order that pays out the fill chain's fee token leaves out of the book.
 * A cross-chain fill pays the escrow-release dispatch fee in that token from the same wallet, so
 * an order sized to the whole balance leaves nothing to pay it with and every fill of it is
 * skipped. Fee tokens are dollar stablecoins, so this is $10.
 */
export const LIMIT_ORDER_FEE_RESERVE_TOKENS = 10n
