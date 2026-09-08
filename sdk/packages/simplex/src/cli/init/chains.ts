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
		explorerUrl: "https://etherscan.io",
	},
	{
		chainId: 42161,
		stateMachineId: "EVM-42161",
		label: "Arbitrum",
		network: "mainnet",
		alchemySubdomain: "arb-mainnet",
		explorerUrl: "https://arbiscan.io",
	},
	{
		chainId: 8453,
		stateMachineId: "EVM-8453",
		label: "Base",
		network: "mainnet",
		alchemySubdomain: "base-mainnet",
		explorerUrl: "https://basescan.org",
	},
	{
		chainId: 137,
		stateMachineId: "EVM-137",
		label: "Polygon",
		network: "mainnet",
		alchemySubdomain: "polygon-mainnet",
		explorerUrl: "https://polygonscan.com",
	},
	{
		chainId: 56,
		stateMachineId: "EVM-56",
		label: "BNB Chain",
		network: "mainnet",
		alchemySubdomain: "bnb-mainnet",
		explorerUrl: "https://bscscan.com",
	},
	{
		chainId: 11155111,
		stateMachineId: "EVM-11155111",
		label: "Sepolia",
		network: "testnet",
		alchemySubdomain: "eth-sepolia",
		explorerUrl: "https://sepolia.etherscan.io",
	},
	{
		chainId: 421614,
		stateMachineId: "EVM-421614",
		label: "Arbitrum Sepolia",
		network: "testnet",
		alchemySubdomain: "arb-sepolia",
		explorerUrl: "https://sepolia.arbiscan.io",
	},
	{
		chainId: 84532,
		stateMachineId: "EVM-84532",
		label: "Base Sepolia",
		network: "testnet",
		alchemySubdomain: "base-sepolia",
		explorerUrl: "https://sepolia.basescan.org",
	},
	{
		chainId: 80002,
		stateMachineId: "EVM-80002",
		label: "Polygon Amoy",
		network: "testnet",
		alchemySubdomain: "polygon-amoy",
		explorerUrl: "https://amoy.polygonscan.com",
	},
	{
		chainId: 97,
		stateMachineId: "EVM-97",
		label: "BSC Chapel",
		network: "testnet",
		alchemySubdomain: "bnb-testnet",
		explorerUrl: "https://testnet.bscscan.com",
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
