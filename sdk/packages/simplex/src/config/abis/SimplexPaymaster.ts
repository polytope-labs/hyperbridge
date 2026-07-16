export const SIMPLEX_PAYMASTER_ABI = [
	{
		inputs: [],
		name: "getRegisteredTokens",
		outputs: [{ name: "", type: "address[]" }],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "treasury",
		outputs: [{ name: "", type: "address" }],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{ name: "token", type: "address" },
			{ name: "amountIn", type: "uint256" },
		],
		name: "swapAndDeposit",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
] as const
