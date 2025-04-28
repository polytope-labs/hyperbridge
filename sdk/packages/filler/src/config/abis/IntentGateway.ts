export const INTENT_GATEWAY_ABI = [
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
		name: "UnauthorizedCall",
		type: "error",
	},
	{
		inputs: [],
		name: "UnexpectedCall",
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
				internalType: "bytes32",
				name: "gateway",
				type: "bytes32",
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
				name: "sourceChain",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "destChain",
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
					{
						internalType: "bytes32",
						name: "beneficiary",
						type: "bytes32",
					},
				],
				indexed: false,
				internalType: "struct PaymentInfo[]",
				name: "outputs",
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
				indexed: false,
				internalType: "bytes",
				name: "callData",
				type: "bytes",
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
						name: "sourceChain",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destChain",
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
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
						],
						internalType: "struct PaymentInfo[]",
						name: "outputs",
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
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						internalType: "bytes",
						name: "callData",
						type: "bytes",
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
						name: "sourceChain",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destChain",
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
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
						],
						internalType: "struct PaymentInfo[]",
						name: "outputs",
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
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						internalType: "bytes",
						name: "callData",
						type: "bytes",
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
				components: [
					{
						components: [
							{
								internalType: "bytes",
								name: "source",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "dest",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "nonce",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "from",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "to",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "timeoutTimestamp",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "body",
								type: "bytes",
							},
						],
						internalType: "struct PostRequest",
						name: "request",
						type: "tuple",
					},
					{
						internalType: "address",
						name: "relayer",
						type: "address",
					},
				],
				internalType: "struct IncomingPostRequest",
				name: "incoming",
				type: "tuple",
			},
		],
		name: "onAccept",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						components: [
							{
								components: [
									{
										internalType: "bytes",
										name: "source",
										type: "bytes",
									},
									{
										internalType: "bytes",
										name: "dest",
										type: "bytes",
									},
									{
										internalType: "uint64",
										name: "nonce",
										type: "uint64",
									},
									{
										internalType: "address",
										name: "from",
										type: "address",
									},
									{
										internalType: "uint64",
										name: "timeoutTimestamp",
										type: "uint64",
									},
									{
										internalType: "bytes[]",
										name: "keys",
										type: "bytes[]",
									},
									{
										internalType: "uint64",
										name: "height",
										type: "uint64",
									},
									{
										internalType: "bytes",
										name: "context",
										type: "bytes",
									},
								],
								internalType: "struct GetRequest",
								name: "request",
								type: "tuple",
							},
							{
								components: [
									{
										internalType: "bytes",
										name: "key",
										type: "bytes",
									},
									{
										internalType: "bytes",
										name: "value",
										type: "bytes",
									},
								],
								internalType: "struct StorageValue[]",
								name: "values",
								type: "tuple[]",
							},
						],
						internalType: "struct GetResponse",
						name: "response",
						type: "tuple",
					},
					{
						internalType: "address",
						name: "relayer",
						type: "address",
					},
				],
				internalType: "struct IncomingGetResponse",
				name: "incoming",
				type: "tuple",
			},
		],
		name: "onGetResponse",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "nonce",
						type: "uint64",
					},
					{
						internalType: "address",
						name: "from",
						type: "address",
					},
					{
						internalType: "uint64",
						name: "timeoutTimestamp",
						type: "uint64",
					},
					{
						internalType: "bytes[]",
						name: "keys",
						type: "bytes[]",
					},
					{
						internalType: "uint64",
						name: "height",
						type: "uint64",
					},
					{
						internalType: "bytes",
						name: "context",
						type: "bytes",
					},
				],
				internalType: "struct GetRequest",
				name: "",
				type: "tuple",
			},
		],
		name: "onGetTimeout",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						internalType: "bytes",
						name: "source",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "nonce",
						type: "uint64",
					},
					{
						internalType: "bytes",
						name: "from",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "to",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeoutTimestamp",
						type: "uint64",
					},
					{
						internalType: "bytes",
						name: "body",
						type: "bytes",
					},
				],
				internalType: "struct PostRequest",
				name: "",
				type: "tuple",
			},
		],
		name: "onPostRequestTimeout",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						components: [
							{
								components: [
									{
										internalType: "bytes",
										name: "source",
										type: "bytes",
									},
									{
										internalType: "bytes",
										name: "dest",
										type: "bytes",
									},
									{
										internalType: "uint64",
										name: "nonce",
										type: "uint64",
									},
									{
										internalType: "bytes",
										name: "from",
										type: "bytes",
									},
									{
										internalType: "bytes",
										name: "to",
										type: "bytes",
									},
									{
										internalType: "uint64",
										name: "timeoutTimestamp",
										type: "uint64",
									},
									{
										internalType: "bytes",
										name: "body",
										type: "bytes",
									},
								],
								internalType: "struct PostRequest",
								name: "request",
								type: "tuple",
							},
							{
								internalType: "bytes",
								name: "response",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "timeoutTimestamp",
								type: "uint64",
							},
						],
						internalType: "struct PostResponse",
						name: "response",
						type: "tuple",
					},
					{
						internalType: "address",
						name: "relayer",
						type: "address",
					},
				],
				internalType: "struct IncomingPostResponse",
				name: "",
				type: "tuple",
			},
		],
		name: "onPostResponse",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				components: [
					{
						components: [
							{
								internalType: "bytes",
								name: "source",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "dest",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "nonce",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "from",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "to",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "timeoutTimestamp",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "body",
								type: "bytes",
							},
						],
						internalType: "struct PostRequest",
						name: "request",
						type: "tuple",
					},
					{
						internalType: "bytes",
						name: "response",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeoutTimestamp",
						type: "uint64",
					},
				],
				internalType: "struct PostResponse",
				name: "",
				type: "tuple",
			},
		],
		name: "onPostResponseTimeout",
		outputs: [],
		stateMutability: "nonpayable",
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
						name: "sourceChain",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "destChain",
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
							{
								internalType: "bytes32",
								name: "beneficiary",
								type: "bytes32",
							},
						],
						internalType: "struct PaymentInfo[]",
						name: "outputs",
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
						internalType: "struct TokenInfo[]",
						name: "inputs",
						type: "tuple[]",
					},
					{
						internalType: "bytes",
						name: "callData",
						type: "bytes",
					},
				],
				internalType: "struct Order",
				name: "order",
				type: "tuple",
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
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "to",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "body",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "payer",
						type: "address",
					},
				],
				internalType: "struct DispatchPost",
				name: "request",
				type: "tuple",
			},
		],
		name: "quote",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "height",
						type: "uint64",
					},
					{
						internalType: "bytes[]",
						name: "keys",
						type: "bytes[]",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "bytes",
						name: "context",
						type: "bytes",
					},
				],
				internalType: "struct DispatchGet",
				name: "request",
				type: "tuple",
			},
		],
		name: "quote",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						components: [
							{
								internalType: "bytes",
								name: "source",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "dest",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "nonce",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "from",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "to",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "timeoutTimestamp",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "body",
								type: "bytes",
							},
						],
						internalType: "struct PostRequest",
						name: "request",
						type: "tuple",
					},
					{
						internalType: "bytes",
						name: "response",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "payer",
						type: "address",
					},
				],
				internalType: "struct DispatchPostResponse",
				name: "response",
				type: "tuple",
			},
		],
		name: "quote",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "to",
						type: "bytes",
					},
					{
						internalType: "bytes",
						name: "body",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "payer",
						type: "address",
					},
				],
				internalType: "struct DispatchPost",
				name: "request",
				type: "tuple",
			},
		],
		name: "quoteNative",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						components: [
							{
								internalType: "bytes",
								name: "source",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "dest",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "nonce",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "from",
								type: "bytes",
							},
							{
								internalType: "bytes",
								name: "to",
								type: "bytes",
							},
							{
								internalType: "uint64",
								name: "timeoutTimestamp",
								type: "uint64",
							},
							{
								internalType: "bytes",
								name: "body",
								type: "bytes",
							},
						],
						internalType: "struct PostRequest",
						name: "request",
						type: "tuple",
					},
					{
						internalType: "bytes",
						name: "response",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "payer",
						type: "address",
					},
				],
				internalType: "struct DispatchPostResponse",
				name: "request",
				type: "tuple",
			},
		],
		name: "quoteNative",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "uint64",
						name: "height",
						type: "uint64",
					},
					{
						internalType: "bytes[]",
						name: "keys",
						type: "bytes[]",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "bytes",
						name: "context",
						type: "bytes",
					},
				],
				internalType: "struct DispatchGet",
				name: "request",
				type: "tuple",
			},
		],
		name: "quoteNative",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
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
						internalType: "address",
						name: "host",
						type: "address",
					},
					{
						internalType: "address",
						name: "dispatcher",
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
