import externalLinks from "../../config/external-links.json"

export type InitNetwork = "mainnet" | "testnet"

export interface InitChainMeta {
	chainId: number
	stateMachineId: string
	label: string
	network: InitNetwork
	/** Subdomain of `<subdomain>.g.alchemy.com`; undefined when Alchemy doesn't serve the chain. */
	alchemySubdomain?: string
	/** Block explorer base URL (no trailing slash) for transaction links. */
	explorerUrl?: string
	/** Short operator-facing caveat shown beside the chain in the wizards (e.g. a paymaster limitation). */
	note?: string
	/**
	 * Public endpoints the wizards offer as the chain's starting RPC set, so an
	 * operator needs no RPC provider account to run. See {@link FREE_RPC_URLS}.
	 */
	defaultRpcUrls?: string[]
}

/**
 * Public RPC endpoints that serve the filler's gateway scan, per chain.
 *
 * Every URL here scanned mainnet for 20 minutes on its own, then ran for 19
 * hours in a quorum without once being benched for rate limiting or for lagging
 * the quorum head. Endpoints that were chronically benched over that run are
 * deliberately absent — chiefly `*.rpc.sentio.xyz`, which sat behind the head
 * for 8961 scans and so never cast a vote, and `thirdweb`, `blockmachine` and
 * `swiftnodes`, which were rate-limited in the hundreds.
 *
 * One endpoint per operator. `validateRpcUrls` only rejects a repeated
 * hostname, but two URLs from one provider are one opinion — they agree with
 * each other by construction, so counting both inflates the denominator of a
 * threshold that exists to survive a provider being wrong. That is why
 * Tenderly's `<chain>.gateway.tenderly.co` and `gateway.tenderly.co/public/
 * <chain>` never both appear, and why Polygon is five endpoints rather than six.
 *
 * No URL carries an API key. A keyed public endpoint would put every operator
 * on one shared quota, and the keys published on endpoint directories rot.
 *
 * These are a starting point, not a guarantee: free endpoints come and go, and
 * an operator with a provider account should use it. Both wizards let the list
 * be edited before the config is written.
 */
const FREE_RPC_URLS: Record<number, string[]> = {
	1: [
		"https://mainnet.gateway.tenderly.co",
		"https://rpc.mevblocker.io",
		"https://ethereum-rpc.publicnode.com",
		"https://ethereum.public.blockpi.network/v1/rpc/public",
		"https://eth.blockrazor.xyz",
		"https://eth.api.pocket.network",
		"https://ethereum-public.nodies.app",
		"https://eth-mainnet.public.blastapi.io",
		"https://public-eth.nownodes.io",
	],
	56: [
		"https://bsc-rpc.publicnode.com",
		"https://public-bsc-mainnet.fastnode.io",
		"https://public-bsc.nownodes.io",
		"https://bsc.blockrazor.xyz",
		"https://rpc-bsc.48.club",
		"https://bsc.api.pocket.network",
	],
	137: [
		"https://polygon-bor-rpc.publicnode.com",
		"https://polygon.drpc.org",
		"https://polygon.gateway.tenderly.co",
		"https://rpc.private.mev-x.com/polygon",
		"https://poly.api.pocket.network",
	],
	8453: [
		"https://base-rpc.publicnode.com",
		"https://base.gateway.tenderly.co",
		"https://base.public.blockpi.network/v1/rpc/public",
		"https://base.rpc.blxrbdn.com",
		"https://rpc.baseazul.dev",
		"https://mainnet.base.org",
		"https://base-public.nodies.app",
	],
	42161: [
		"https://arbitrum-one-rpc.publicnode.com",
		"https://arbitrum.gateway.tenderly.co",
		"https://arbitrum.drpc.org",
		"https://public-arb-mainnet.fastnode.io",
		"https://arb-one.api.pocket.network",
		"https://arbitrum-one-public.nodies.app",
		"https://arb1.arbitrum.io/rpc",
	],
}

const CHAIN_NATIVE_SYMBOLS: Record<number, string> = {
	1: "ETH",
	56: "BNB",
	97: "BNB",
	100: "xDAI",
	130: "ETH",
	137: "MATIC",
	250: "FTM",
	43114: "AVAX",
	42161: "ETH",
	8453: "ETH",
	10: "ETH",
	80002: "MATIC",
	84532: "ETH",
	11155111: "ETH",
	421614: "ETH",
}

/** Display symbol for a chain's native gas asset. */
export function nativeTokenSymbol(chainId: number): string {
	return CHAIN_NATIVE_SYMBOLS[chainId] ?? "ETH"
}

export const INIT_CHAINS: InitChainMeta[] = [
	{
		chainId: 1,
		stateMachineId: "EVM-1",
		label: "Ethereum",
		network: "mainnet",
		alchemySubdomain: "eth-mainnet",
		explorerUrl: externalLinks.chainExplorers.ethereum,
		defaultRpcUrls: FREE_RPC_URLS[1],
	},
	{
		chainId: 42161,
		stateMachineId: "EVM-42161",
		label: "Arbitrum",
		network: "mainnet",
		alchemySubdomain: "arb-mainnet",
		explorerUrl: externalLinks.chainExplorers.arbitrum,
		defaultRpcUrls: FREE_RPC_URLS[42161],
	},
	{
		chainId: 8453,
		stateMachineId: "EVM-8453",
		label: "Base",
		network: "mainnet",
		alchemySubdomain: "base-mainnet",
		explorerUrl: externalLinks.chainExplorers.base,
		defaultRpcUrls: FREE_RPC_URLS[8453],
	},
	{
		chainId: 137,
		stateMachineId: "EVM-137",
		label: "Polygon",
		network: "mainnet",
		alchemySubdomain: "polygon-mainnet",
		explorerUrl: externalLinks.chainExplorers.polygon,
		defaultRpcUrls: FREE_RPC_URLS[137],
	},
	{
		chainId: 56,
		stateMachineId: "EVM-56",
		label: "BNB Chain",
		network: "mainnet",
		alchemySubdomain: "bnb-mainnet",
		explorerUrl: externalLinks.chainExplorers.bnb,
		defaultRpcUrls: FREE_RPC_URLS[56],
	},
	{
		chainId: 11155111,
		stateMachineId: "EVM-11155111",
		label: "Sepolia",
		network: "testnet",
		alchemySubdomain: "eth-sepolia",
		explorerUrl: externalLinks.chainExplorers.sepolia,
	},
	{
		chainId: 421614,
		stateMachineId: "EVM-421614",
		label: "Arbitrum Sepolia",
		network: "testnet",
		alchemySubdomain: "arb-sepolia",
		explorerUrl: externalLinks.chainExplorers.arbitrumSepolia,
	},
	{
		chainId: 84532,
		stateMachineId: "EVM-84532",
		label: "Base Sepolia",
		network: "testnet",
		alchemySubdomain: "base-sepolia",
		explorerUrl: externalLinks.chainExplorers.baseSepolia,
	},
	{
		chainId: 80002,
		stateMachineId: "EVM-80002",
		label: "Polygon Amoy",
		network: "testnet",
		alchemySubdomain: "polygon-amoy",
		explorerUrl: externalLinks.chainExplorers.polygonAmoy,
	},
	{
		chainId: 97,
		stateMachineId: "EVM-97",
		label: "BSC Chapel",
		network: "testnet",
		alchemySubdomain: "bnb-testnet",
		explorerUrl: externalLinks.chainExplorers.bnbChapel,
	},
]

export const HYPERBRIDGE_WS_DEFAULTS: Record<InitNetwork, string> = {
	mainnet: "wss://nexus.rpc.polytope.technology",
	testnet: "wss://gargantua.rpc.polytope.technology",
}

export function chainsForNetwork(network: InitNetwork): InitChainMeta[] {
	return INIT_CHAINS.filter((chain) => chain.network === network)
}

export function chainByChainId(chainId: number): InitChainMeta | undefined {
	return INIT_CHAINS.find((chain) => chain.chainId === chainId)
}

export function chainByAlchemySubdomain(subdomain: string): InitChainMeta | undefined {
	return INIT_CHAINS.find((chain) => chain.alchemySubdomain === subdomain)
}
