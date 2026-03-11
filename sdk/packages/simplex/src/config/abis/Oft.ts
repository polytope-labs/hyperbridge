export const OFT_ABI = [
	{
		name: "quoteOFT",
		type: "function",
		stateMutability: "view",
		inputs: [
			{
				name: "sendParam",
				type: "tuple",
				components: [
					{ name: "dstEid", type: "uint32" },
					{ name: "to", type: "bytes32" },
					{ name: "amountLD", type: "uint256" },
					{ name: "minAmountLD", type: "uint256" },
					{ name: "extraOptions", type: "bytes" },
					{ name: "composeMsg", type: "bytes" },
					{ name: "oftCmd", type: "bytes" },
				],
			},
		],
		outputs: [
			{
				name: "oftLimit",
				type: "tuple",
				components: [
					{ name: "minAmountLD", type: "uint256" },
					{ name: "maxAmountLD", type: "uint256" },
				],
			},
			{
				name: "oftFeeDetails",
				type: "tuple[]",
				components: [
					{ name: "feeAmountLD", type: "int256" },
					{ name: "description", type: "string" },
				],
			},
			{
				name: "oftReceipt",
				type: "tuple",
				components: [
					{ name: "amountSentLD", type: "uint256" },
					{ name: "amountReceivedLD", type: "uint256" },
				],
			},
		],
	},
	{
		name: "quoteSend",
		type: "function",
		stateMutability: "view",
		inputs: [
			{
				name: "sendParam",
				type: "tuple",
				components: [
					{ name: "dstEid", type: "uint32" },
					{ name: "to", type: "bytes32" },
					{ name: "amountLD", type: "uint256" },
					{ name: "minAmountLD", type: "uint256" },
					{ name: "extraOptions", type: "bytes" },
					{ name: "composeMsg", type: "bytes" },
					{ name: "oftCmd", type: "bytes" },
				],
			},
			{ name: "payInLzToken", type: "bool" },
		],
		outputs: [
			{
				name: "msgFee",
				type: "tuple",
				components: [
					{ name: "nativeFee", type: "uint256" },
					{ name: "lzTokenFee", type: "uint256" },
				],
			},
		],
	},
	{
		name: "send",
		type: "function",
		stateMutability: "payable",
		inputs: [
			{
				name: "sendParam",
				type: "tuple",
				components: [
					{ name: "dstEid", type: "uint32" },
					{ name: "to", type: "bytes32" },
					{ name: "amountLD", type: "uint256" },
					{ name: "minAmountLD", type: "uint256" },
					{ name: "extraOptions", type: "bytes" },
					{ name: "composeMsg", type: "bytes" },
					{ name: "oftCmd", type: "bytes" },
				],
			},
			{
				name: "fee",
				type: "tuple",
				components: [
					{ name: "nativeFee", type: "uint256" },
					{ name: "lzTokenFee", type: "uint256" },
				],
			},
			{ name: "refundAddress", type: "address" },
		],
		outputs: [
			{
				name: "msgReceipt",
				type: "tuple",
				components: [
					{ name: "guid", type: "bytes32" },
					{ name: "nonce", type: "uint64" },
					{
						name: "fee",
						type: "tuple",
						components: [
							{ name: "nativeFee", type: "uint256" },
							{ name: "lzTokenFee", type: "uint256" },
						],
					},
				],
			},
			{
				name: "oftReceipt",
				type: "tuple",
				components: [
					{ name: "amountSentLD", type: "uint256" },
					{ name: "amountReceivedLD", type: "uint256" },
				],
			},
		],
	},
] as const
