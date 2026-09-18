/** Orders the engine evaluates concurrently when the config does not say. */
export const DEFAULT_MAX_CONCURRENT_ORDERS = 5

/** How long an orderbook request waits before it is treated as unreachable. */
export const DEFAULT_ORDERBOOK_TIMEOUT_MS = 10_000

/** The orderbook simplex posts to when the config does not name another. */
export const DEFAULT_ORDERBOOK_URL = "https://orderbook.hyperbridge.network/graphql"
