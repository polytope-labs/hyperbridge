/**
 * Minimal Aerodrome / Solidly-style router, pair, and gauge ABIs.
 * Deployments may differ slightly; extend fragments if a chain adds functions.
 */

export const AERODROME_PAIR_ABI = [
	{
		name: "token0",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [{ type: "address" }],
	},
	{
		name: "token1",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [{ type: "address" }],
	},
	{
		name: "stable",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [{ type: "bool" }],
	},
	{
		name: "totalSupply",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [{ type: "uint256" }],
	},
	{
		name: "getReserves",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [
			{ name: "reserve0", type: "uint256" },
			{ name: "reserve1", type: "uint256" },
			{ name: "blockTimestampLast", type: "uint256" },
		],
	},
	{
		name: "balanceOf",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "account", type: "address" }],
		outputs: [{ type: "uint256" }],
	},
	{
		name: "approve",
		type: "function",
		stateMutability: "nonpayable",
		inputs: [
			{ name: "spender", type: "address" },
			{ name: "amount", type: "uint256" },
		],
		outputs: [{ type: "bool" }],
	},
] as const

export const AERODROME_ROUTER_ABI = [
	{
		name: "quoteRemoveLiquidity",
		type: "function",
		stateMutability: "view",
		inputs: [
			{ name: "tokenA", type: "address" },
			{ name: "tokenB", type: "address" },
			{ name: "stable", type: "bool" },
			{ name: "liquidity", type: "uint256" },
		],
		outputs: [
			{ name: "amountA", type: "uint256" },
			{ name: "amountB", type: "uint256" },
		],
	},
	{
		name: "removeLiquidity",
		type: "function",
		stateMutability: "nonpayable",
		inputs: [
			{ name: "tokenA", type: "address" },
			{ name: "tokenB", type: "address" },
			{ name: "stable", type: "bool" },
			{ name: "liquidity", type: "uint256" },
			{ name: "amountAMin", type: "uint256" },
			{ name: "amountBMin", type: "uint256" },
			{ name: "to", type: "address" },
			{ name: "deadline", type: "uint256" },
		],
		outputs: [
			{ name: "amountA", type: "uint256" },
			{ name: "amountB", type: "uint256" },
		],
	},
] as const

export const AERODROME_GAUGE_ABI = [
	{
		name: "withdraw",
		type: "function",
		stateMutability: "nonpayable",
		inputs: [{ name: "amount", type: "uint256" }],
		outputs: [],
	},
	{
		name: "balanceOf",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "account", type: "address" }],
		outputs: [{ type: "uint256" }],
	},
	{
		name: "stakingToken",
		type: "function",
		stateMutability: "view",
		inputs: [],
		outputs: [{ type: "address" }],
	},
] as const
