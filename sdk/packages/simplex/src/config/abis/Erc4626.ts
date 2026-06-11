export const ERC4626_ABI = [
	{
		inputs: [],
		name: "asset",
		outputs: [{ internalType: "address", name: "", type: "address" }],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{ internalType: "uint256", name: "assets", type: "uint256" },
			{ internalType: "address", name: "receiver", type: "address" },
		],
		name: "deposit",
		outputs: [{ internalType: "uint256", name: "shares", type: "uint256" }],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{ internalType: "uint256", name: "assets", type: "uint256" },
			{ internalType: "address", name: "receiver", type: "address" },
			{ internalType: "address", name: "owner", type: "address" },
		],
		name: "withdraw",
		outputs: [{ internalType: "uint256", name: "shares", type: "uint256" }],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{ internalType: "uint256", name: "shares", type: "uint256" },
			{ internalType: "address", name: "receiver", type: "address" },
			{ internalType: "address", name: "owner", type: "address" },
		],
		name: "redeem",
		outputs: [{ internalType: "uint256", name: "assets", type: "uint256" }],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [{ internalType: "address", name: "owner", type: "address" }],
		name: "maxWithdraw",
		outputs: [{ internalType: "uint256", name: "", type: "uint256" }],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [{ internalType: "uint256", name: "shares", type: "uint256" }],
		name: "previewRedeem",
		outputs: [{ internalType: "uint256", name: "", type: "uint256" }],
		stateMutability: "view",
		type: "function",
	},
] as const
