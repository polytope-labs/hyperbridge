const ABI = [
	{
		inputs: [
			{
				internalType: "address",
				name: "admin",
				type: "address",
			},
		],
		stateMutability: "nonpayable",
		type: "constructor",
	},
	{
		inputs: [],
		name: "Cancelled",
		type: "error",
	},
	{
		inputs: [],
		name: "Expired",
		type: "error",
	},
	{
		inputs: [],
		name: "Filled",
		type: "error",
	},
	{
		inputs: [],
		name: "InsufficientNativeToken",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidInput",
		type: "error",
	},
	{
		inputs: [],
		name: "NotExpired",
		type: "error",
	},
	{
		inputs: [],
		name: "Unauthorized",
		type: "error",
	},
	{
		inputs: [],
		name: "UnknownOrder",
		type: "error",
	},
	{
		inputs: [],
		name: "WrongChain",
		type: "error",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "address",
				name: "token",
				type: "address",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "amount",
				type: "uint256",
			},
		],
		name: "DustCollected",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "address",
				name: "token",
				type: "address",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "amount",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "address",
				name: "beneficiary",
				type: "address",
			},
		],
		name: "DustSwept",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "EscrowRefunded",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "EscrowReleased",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "bytes",
				name: "stateMachineId",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "address",
				name: "gateway",
				type: "address",
			},
		],
		name: "NewDeploymentAdded",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
			{
				indexed: false,
				internalType: "address",
				name: "filler",
				type: "address",
			},
		],
		name: "OrderFilled",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "bytes32",
				name: "user",
				type: "bytes32",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "source",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "destination",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "deadline",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "nonce",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "fees",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "address",
				name: "session",
				type: "address",
			},
			{
				indexed: false,
				internalType: "bytes32",
				name: "beneficiary",
				type: "bytes32",
			},
			{
				components: [
					{
						internalType: "bytes32",
						name: "token",
						type: "bytes32",
					},
					{
						internalType: "uint256",
						name: "amount",
						type: "uint256",
					},
				],
				indexed: false,
				internalType: "struct TokenInfo[]",
				name: "predispatch",
				type: "tuple[]",
			},
			{
				components: [
					{
						internalType: "bytes32",
						name: "token",
						type: "bytes32",
					},
					{
						internalType: "uint256",
						name: "amount",
						type: "uint256",
					},
				],
				indexed: false,
				internalType: "struct TokenInfo[]",
				name: "inputs",
				type: "tuple[]",
			},
			{
				components: [
					{
						internalType: "bytes32",
						name: "token",
						type: "bytes32",
					},
					{
						internalType: "uint256",
						name: "amount",
						type: "uint256",
					},
				],
				indexed: false,
				internalType: "struct TokenInfo[]",
				name: "outputs",
				type: "tuple[]",
			},
		],
		name: "OrderPlaced",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				components: [
					{
						internalType: "address",
						name: "host",
						type: "address",
					},
					{
						internalType: "address",
						name: "dispatcher",
						type: "address",
					},
					{
						internalType: "bool",
						name: "solverSelection",
						type: "bool",
					},
					{
						internalType: "uint256",
						name: "surplusShareBps",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "protocolFeeBps",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "priceOracle",
						type: "address",
					},
				],
				indexed: false,
				internalType: "struct Params",
				name: "previous",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "address",
						name: "host",
						type: "address",
					},
					{
						internalType: "address",
						name: "dispatcher",
						type: "address",
					},
					{
						internalType: "bool",
						name: "solverSelection",
						type: "bool",
					},
					{
						internalType: "uint256",
						name: "surplusShareBps",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "protocolFeeBps",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "priceOracle",
						type: "address",
					},
				],
				indexed: false,
				internalType: "struct Params",
				name: "current",
				type: "tuple",
			},
		],
		name: "ParamsUpdated",
		type: "event",
	},
	{
		inputs: [],
		name: "DOMAIN_SEPARATOR",
		outputs: [
			{
				internalType: "bytes32",
				name: "",
				type: "bytes32",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "SELECT_SOLVER_TYPEHASH",
		outputs: [
			{
				internalType: "bytes32",
				name: "",
				type: "bytes32",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "calculateCommitmentSlotHash",
		outputs: [
			{
				internalType: "bytes",
				name: "",
				type: "bytes",
			},
		],
		stateMutability: "pure",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "user",
						type: "bytes32",
					},
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destination",
						type: "bytes",
					},
					{
						internalType: "uint256",
						name: "deadline",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "nonce",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "fees",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "session",
						type: "address",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "assets",
						type: "tuple[]",
					},
					{
						internalType: "bytes",
						name: "call",
						type: "bytes",
					},
				],
				internalType: "struct DispatchInfo",
				name: "predispatch",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "bytes32",
						name: "token",
						type: "bytes32",
					},
					{
						internalType: "uint256",
						name: "amount",
						type: "uint256",
					},
				],
				internalType: "struct TokenInfo[]",
				name: "inputs",
				type: "tuple[]",
			},
			{
				components: [
					{
						internalType: "bytes32",
						name: "beneficiary",
						type: "bytes32",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "assets",
						type: "tuple[]",
					},
					{
						internalType: "bytes",
						name: "call",
						type: "bytes",
					},
				],
				internalType: "struct PaymentInfo",
				name: "output",
				type: "tuple",
			},
		],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "user",
						type: "bytes32",
					},
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destination",
						type: "bytes",
					},
					{
						internalType: "uint256",
						name: "deadline",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "nonce",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "fees",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "session",
						type: "address",
					},
					{
						components: [
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct DispatchInfo",
						name: "predispatch",
						type: "tuple",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct PaymentInfo",
						name: "output",
						type: "tuple",
					},
				],
				internalType: "struct Order",
				name: "order",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "relayerFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "height",
						type: "uint256",
					},
				],
				internalType: "struct CancelOptions",
				name: "options",
				type: "tuple",
			},
		],
		name: "cancelOrder",
		outputs: [],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "user",
						type: "bytes32",
					},
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destination",
						type: "bytes",
					},
					{
						internalType: "uint256",
						name: "deadline",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "nonce",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "fees",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "session",
						type: "address",
					},
					{
						components: [
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct DispatchInfo",
						name: "predispatch",
						type: "tuple",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct PaymentInfo",
						name: "output",
						type: "tuple",
					},
				],
				internalType: "struct Order",
				name: "order",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "relayerFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "nativeDispatchFee",
						type: "uint256",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "outputs",
						type: "tuple[]",
					},
				],
				internalType: "struct FillOptions",
				name: "options",
				type: "tuple",
			},
		],
		name: "fillOrder",
		outputs: [],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [],
		name: "host",
		outputs: [
			{
				internalType: "address",
				name: "",
				type: "address",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "bytes",
				name: "stateMachineId",
				type: "bytes",
			},
		],
		name: "instance",
		outputs: [
			{
				internalType: "address",
				name: "",
				type: "address",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "params",
		outputs: [
			{
				components: [
					{
						internalType: "address",
						name: "host",
						type: "address",
					},
					{
						internalType: "address",
						name: "dispatcher",
						type: "address",
					},
					{
						internalType: "bool",
						name: "solverSelection",
						type: "bool",
					},
					{
						internalType: "uint256",
						name: "surplusShareBps",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "protocolFeeBps",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "priceOracle",
						type: "address",
					},
				],
				internalType: "struct Params",
				name: "",
				type: "tuple",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "user",
						type: "bytes32",
					},
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destination",
						type: "bytes",
					},
					{
						internalType: "uint256",
						name: "deadline",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "nonce",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "fees",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "session",
						type: "address",
					},
					{
						components: [
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct DispatchInfo",
						name: "predispatch",
						type: "tuple",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "token",
								type: "bytes32",
							},
							{
								internalType: "uint256",
								name: "amount",
								type: "uint256",
							},
						],
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
							{
								components: [
									{
										internalType: "bytes32",
										name: "token",
										type: "bytes32",
									},
									{
										internalType: "uint256",
										name: "amount",
										type: "uint256",
									},
								],
								internalType: "struct TokenInfo[]",
								name: "assets",
								type: "tuple[]",
							},
							{
								internalType: "bytes",
								name: "call",
								type: "bytes",
							},
						],
						internalType: "struct PaymentInfo",
						name: "output",
						type: "tuple",
					},
				],
				internalType: "struct Order",
				name: "order",
				type: "tuple",
			},
			{
				internalType: "bytes32",
				name: "graffiti",
				type: "bytes32",
			},
		],
		name: "placeOrder",
		outputs: [],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "commitment",
						type: "bytes32",
					},
					{
						internalType: "address",
						name: "solver",
						type: "address",
					},
					{
						internalType: "bytes",
						name: "signature",
						type: "bytes",
					},
				],
				internalType: "struct SelectOptions",
				name: "options",
				type: "tuple",
			},
		],
		name: "select",
		outputs: [
			{
				internalType: "address",
				name: "sessionKey",
				type: "address",
			},
		],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "address",
						name: "host",
						type: "address",
					},
					{
						internalType: "address",
						name: "dispatcher",
						type: "address",
					},
					{
						internalType: "bool",
						name: "solverSelection",
						type: "bool",
					},
					{
						internalType: "uint256",
						name: "surplusShareBps",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "protocolFeeBps",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "priceOracle",
						type: "address",
					},
				],
				internalType: "struct Params",
				name: "p",
				type: "tuple",
			},
		],
		name: "setParams",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		stateMutability: "payable",
		type: "receive",
	},
] as const

export default { ABI }
