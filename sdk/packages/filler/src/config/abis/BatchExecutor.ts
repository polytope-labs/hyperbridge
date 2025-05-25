export const BATCH_EXECUTOR_ABI = [
	{
		inputs: [
			{
				components: [
					{
						internalType: "address",
						name: "to",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "value",
						type: "uint256",
					},
					{
						internalType: "bytes",
						name: "data",
						type: "bytes",
					},
				],
				internalType: "struct BatchExecutor.Call[]",
				name: "calls",
				type: "tuple[]",
			},
		],
		name: "execute",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
] as const
