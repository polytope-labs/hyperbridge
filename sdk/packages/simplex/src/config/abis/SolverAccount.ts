export const SOLVER_ACCOUNT_ABI = [
	{
		inputs: [
			{ name: "orderId", type: "bytes32" },
			{ name: "cap", type: "uint256" },
			{ name: "token", type: "address" },
			{ name: "approved", type: "uint256" },
			{ name: "fee", type: "uint256" },
		],
		name: "debitOrder",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [{ name: "orderId", type: "bytes32" }],
		name: "spent",
		outputs: [{ name: "", type: "uint256" }],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [{ name: "sender", type: "address" }],
		name: "AccountUnauthorized",
		type: "error",
	},
	{
		inputs: [
			{ name: "orderId", type: "bytes32" },
			{ name: "total", type: "uint256" },
			{ name: "cap", type: "uint256" },
		],
		name: "LimitOrderExceeded",
		type: "error",
	},
] as const
