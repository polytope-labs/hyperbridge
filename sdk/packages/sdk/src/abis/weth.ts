const ABI = [
	{
		name: "deposit",
		type: "function",
		stateMutability: "payable",
		inputs: [],
		outputs: [],
	},
	{
		name: "withdraw",
		type: "function",
		stateMutability: "nonpayable",
		inputs: [{ name: "wad", type: "uint256" }],
		outputs: [],
	},
] as const

export default { ABI }
