const ABI = [
	{
		inputs: [
			{ name: "sender", type: "address" },
			{ name: "key", type: "uint192" },
		],
		name: "getNonce",
		outputs: [{ name: "nonce", type: "uint256" }],
		stateMutability: "view",
		type: "function",
	},
] as const

export default { ABI }
