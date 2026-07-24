/**
 * Registry of well-known, keyless public JSON-RPC endpoints for the EVM networks
 * Simplex supports. These are merged into the *quorum* provider set alongside the
 * operator's own endpoints (see `FillerConfigService.getQuorumRpcUrls`) so that
 * order detection and cross-chain confirmation counting never rest on a single
 * provider's — or a single organisation's — view of the chain.
 *
 * Selection criteria: each chain lists endpoints operated by organisationally
 * independent providers (the network's official gateway where one exists, plus
 * PublicNode/Allnodes, dRPC and 1RPC), all reachable without an API key. The
 * quorum threshold is `floor(2N/3) + 1`, so four public endpoints on top of a
 * single operator endpoint (N=5, threshold 4) tolerate one faulty or lying
 * provider; every additional operator endpoint raises the tolerance further.
 *
 * Public endpoints are only ever used for quorum-checked reads (`eth_getLogs`,
 * `eth_blockNumber`, `eth_getTransactionReceipt`). Latency-sensitive and
 * write paths (gas estimation, transaction submission, balance reads) stay on
 * the operator's configured endpoints.
 */
export const PUBLIC_RPC_URLS: Record<number, readonly string[]> = {
	// Ethereum Mainnet
	1: [
		"https://ethereum-rpc.publicnode.com",
		"https://eth.drpc.org",
		"https://1rpc.io/eth",
		"https://eth.meowrpc.com",
	],
	// BNB Smart Chain
	56: [
		"https://bsc-dataseed.bnbchain.org",
		"https://bsc-rpc.publicnode.com",
		"https://bsc.drpc.org",
		"https://1rpc.io/bnb",
	],
	// Polygon PoS
	137: [
		"https://polygon-bor-rpc.publicnode.com",
		"https://polygon.drpc.org",
		"https://1rpc.io/matic",
		"https://polygon.gateway.tenderly.co",
	],
	// Base
	8453: [
		"https://mainnet.base.org",
		"https://base-rpc.publicnode.com",
		"https://base.drpc.org",
		"https://1rpc.io/base",
	],
	// Arbitrum One
	42161: [
		"https://arb1.arbitrum.io/rpc",
		"https://arbitrum-one-rpc.publicnode.com",
		"https://arbitrum.drpc.org",
		"https://1rpc.io/arb",
	],
}
for (const urls of Object.values(PUBLIC_RPC_URLS)) Object.freeze(urls)
Object.freeze(PUBLIC_RPC_URLS)

/**
 * Public RPC endpoints for a chain, or an empty array for networks without a
 * registry entry (testnets, chains Simplex does not curate endpoints for).
 */
export function getPublicRpcUrls(chainId: number): readonly string[] {
	return PUBLIC_RPC_URLS[chainId] ?? []
}
