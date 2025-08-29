const DEFAULT_TOKEN_UPDATE_FREQUENCY = 600 as const // 10 minutes

/**
 * Token configuration interface.
 */
export interface TokenConfig {
	name: string
	symbol: string
	address?: string // Optional - zero address for native tokens
	updateFrequencySeconds: number
}

export const TOKEN_REGISTRY: TokenConfig[] = [
	// Native/Gas tokens
	{
		name: "ETH",
		symbol: "ETH",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Polkadot",
		symbol: "DOT",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Gnosis xDAI",
		symbol: "XDAI",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},

	// Major stablecoins
	{
		name: "USD coin",
		symbol: "USDC",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Tether USD",
		symbol: "USDT",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Maker DAI",
		symbol: "DAI",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	// {
	// 	name: "USDH",
	// 	symbol: "USDH",
	// 	updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	// },

	// Substrate tokens
	{
		name: "Bifrost",
		symbol: "BNC",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Cere Network",
		symbol: "CERE",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},

	// Parachain tokens
	{
		name: "Moonbeam",
		symbol: "GLMR",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Astar",
		symbol: "ASTR",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},

	// Voucher/Liquid staking tokens
	{
		name: "Voucher DOT",
		symbol: "vDOT",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Voucher BNC",
		symbol: "vBNC",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Bifrost Voucher ASTR",
		symbol: "vASTR",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
	{
		name: "Voucher GLMR",
		symbol: "vGLMR",
		updateFrequencySeconds: DEFAULT_TOKEN_UPDATE_FREQUENCY,
	},
]
