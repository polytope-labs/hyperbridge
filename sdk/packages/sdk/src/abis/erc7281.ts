const ABI = [
	{
		name: "execute",
		type: "function",
		inputs: [
			{ name: "mode", type: "bytes32" },
			{ name: "executionData", type: "bytes" },
		],
		outputs: [],
	},
	{
		name: "Call",
		type: "tuple",
		components: [
			{ name: "target", type: "address" },
			{ name: "value", type: "uint256" },
			{ name: "data", type: "bytes" },
		],
	},
] as const

export default { ABI }
