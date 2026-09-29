export const SOLVER_ACCOUNT_ABI = [
	{
		inputs: [
			{ name: "budgetId", type: "bytes32" },
			{ name: "cap", type: "uint256" },
			{ name: "token", type: "address" },
			{ name: "approved", type: "uint256" },
			{ name: "fee", type: "uint256" },
		],
		name: "settleBudget",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [{ name: "budgetId", type: "bytes32" }],
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
			{ name: "budgetId", type: "bytes32" },
			{ name: "total", type: "uint256" },
			{ name: "cap", type: "uint256" },
		],
		name: "BudgetExceeded",
		type: "error",
	},
] as const
