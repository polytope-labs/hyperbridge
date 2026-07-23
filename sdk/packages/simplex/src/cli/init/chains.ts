export type InitNetwork = "mainnet" | "testnet"

export interface InitChainMeta {
	chainId: number
	stateMachineId: string
	label: string
	network: InitNetwork
	/** Subdomain of `<subdomain>.g.alchemy.com`; undefined when Alchemy doesn't serve the chain. */
	alchemySubdomain?: string
	/** Extra caveat surfaced next to the chain in prompts. */
	note?: string
}

export const INIT_CHAINS: InitChainMeta[] = [
	{
		chainId: 1,
		stateMachineId: "EVM-1",
		label: "Ethereum",
		network: "mainnet",
		alchemySubdomain: "eth-mainnet",
	},
	{
		chainId: 42161,
		stateMachineId: "EVM-42161",
		label: "Arbitrum",
		network: "mainnet",
		alchemySubdomain: "arb-mainnet",
	},
	{
		chainId: 8453,
		stateMachineId: "EVM-8453",
		label: "Base",
		network: "mainnet",
		alchemySubdomain: "base-mainnet",
	},
	{
		chainId: 137,
		stateMachineId: "EVM-137",
		label: "Polygon",
		network: "mainnet",
		alchemySubdomain: "polygon-mainnet",
	},
	{
		chainId: 56,
		stateMachineId: "EVM-56",
		label: "BNB Chain",
		network: "mainnet",
		alchemySubdomain: "bnb-mainnet",
		note: "no Circle paymaster — the filler wallet also needs native BNB for gas",
	},
	{
		chainId: 11155111,
		stateMachineId: "EVM-11155111",
		label: "Sepolia",
		network: "testnet",
		alchemySubdomain: "eth-sepolia",
	},
	{
		chainId: 421614,
		stateMachineId: "EVM-421614",
		label: "Arbitrum Sepolia",
		network: "testnet",
		alchemySubdomain: "arb-sepolia",
	},
	{
		chainId: 84532,
		stateMachineId: "EVM-84532",
		label: "Base Sepolia",
		network: "testnet",
		alchemySubdomain: "base-sepolia",
	},
	{
		chainId: 80002,
		stateMachineId: "EVM-80002",
		label: "Polygon Amoy",
		network: "testnet",
		alchemySubdomain: "polygon-amoy",
	},
	{
		chainId: 97,
		stateMachineId: "EVM-97",
		label: "BSC Chapel",
		network: "testnet",
		alchemySubdomain: "bnb-testnet",
		note: "no Circle paymaster — the filler wallet also needs native tBNB for gas",
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
