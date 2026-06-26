// Auto-generated from compiled Solidity contracts

export const HyperFungibleTokenABI = [
	{
		type: "constructor",
		inputs: [
			{
				name: "name",
				type: "string",
				internalType: "string",
			},
			{
				name: "symbol",
				type: "string",
				internalType: "string",
			},
			{
				name: "initialOwner",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "addChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "moduleId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "allowance",
		inputs: [
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "approve",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "balanceOf",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "configure",
		inputs: [
			{
				name: "options",
				type: "tuple",
				internalType: "struct HyperFungibleToken.ConfigOptions",
				components: [
					{
						name: "host",
						type: "address",
						internalType: "address",
					},
					{
						name: "dispatcher",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "decimals",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "uint8",
				internalType: "uint8",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "dispatcher",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "host",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "name",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "string",
				internalType: "string",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "onAccept",
		inputs: [
			{
				name: "incoming",
				type: "tuple",
				internalType: "struct IncomingPostRequest",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct PostRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "to",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "body",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onGetResponse",
		inputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct IncomingGetResponse",
				components: [
					{
						name: "response",
						type: "tuple",
						internalType: "struct GetResponse",
						components: [
							{
								name: "request",
								type: "tuple",
								internalType: "struct GetRequest",
								components: [
									{
										name: "source",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "dest",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "nonce",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "from",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "timeoutTimestamp",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "keys",
										type: "bytes[]",
										internalType: "bytes[]",
									},
									{
										name: "height",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "context",
										type: "bytes",
										internalType: "bytes",
									},
								],
							},
							{
								name: "values",
								type: "tuple[]",
								internalType: "struct StorageValue[]",
								components: [
									{
										name: "key",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "value",
										type: "bytes",
										internalType: "bytes",
									},
								],
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onGetTimeout",
		inputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct GetRequestTimeout",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct GetRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "keys",
								type: "bytes[]",
								internalType: "bytes[]",
							},
							{
								name: "height",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "context",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onPostRequestTimeout",
		inputs: [
			{
				name: "incoming",
				type: "tuple",
				internalType: "struct PostRequestTimeout",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct PostRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "to",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "body",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "owner",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "pause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "paused",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "request",
				type: "tuple",
				internalType: "struct DispatchPost",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "body",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "fee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "payer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "params",
				type: "tuple",
				internalType: "struct HyperFungibleToken.SendParams",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "amount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "relayerFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "data",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "request",
				type: "tuple",
				internalType: "struct DispatchGet",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "height",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "keys",
						type: "bytes[]",
						internalType: "bytes[]",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "fee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "context",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "removeChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "renounceOwnership",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "send",
		inputs: [
			{
				name: "params",
				type: "tuple",
				internalType: "struct HyperFungibleToken.SendParams",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "amount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "relayerFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "data",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "supportedChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [
			{
				name: "",
				type: "bytes",
				internalType: "bytes",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "supportsInterface",
		inputs: [
			{
				name: "interfaceId",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "symbol",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "string",
				internalType: "string",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "totalSupply",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "transfer",
		inputs: [
			{
				name: "to",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "transferFrom",
		inputs: [
			{
				name: "from",
				type: "address",
				internalType: "address",
			},
			{
				name: "to",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "transferOwnership",
		inputs: [
			{
				name: "newOwner",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "unpause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "Approval",
		inputs: [
			{
				name: "owner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "spender",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "OwnershipTransferred",
		inputs: [
			{
				name: "previousOwner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "newOwner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Paused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Received",
		inputs: [
			{
				name: "from",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
			},
			{
				name: "to",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "source",
				type: "string",
				indexed: false,
				internalType: "string",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Refunded",
		inputs: [
			{
				name: "to",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Sent",
		inputs: [
			{
				name: "from",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "to",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
			},
			{
				name: "dest",
				type: "string",
				indexed: false,
				internalType: "string",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "commitment",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Transfer",
		inputs: [
			{
				name: "from",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "to",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Unpaused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "error",
		name: "ERC20InsufficientAllowance",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
			{
				name: "allowance",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "needed",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InsufficientBalance",
		inputs: [
			{
				name: "sender",
				type: "address",
				internalType: "address",
			},
			{
				name: "balance",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "needed",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidApprover",
		inputs: [
			{
				name: "approver",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidReceiver",
		inputs: [
			{
				name: "receiver",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidSender",
		inputs: [
			{
				name: "sender",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidSpender",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "EnforcedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "ExpectedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidAddress",
		inputs: [
			{
				name: "length",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "OwnableInvalidOwner",
		inputs: [
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "OwnableUnauthorizedAccount",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "SafeERC20FailedOperation",
		inputs: [
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "UnauthorizedCall",
		inputs: [],
	},
	{
		type: "error",
		name: "UnauthorizedSource",
		inputs: [],
	},
	{
		type: "error",
		name: "UnexpectedCall",
		inputs: [],
	},
	{
		type: "error",
		name: "UnsupportedChain",
		inputs: [],
	},
] as const

export const WrappedHyperFungibleTokenABI = [
	{
		type: "constructor",
		inputs: [
			{
				name: "initialOwner",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "receive",
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "addChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "moduleId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "configure",
		inputs: [
			{
				name: "options",
				type: "tuple",
				internalType: "struct WrappedHyperFungibleToken.WrappedConfigOptions",
				components: [
					{
						name: "host",
						type: "address",
						internalType: "address",
					},
					{
						name: "dispatcher",
						type: "address",
						internalType: "address",
					},
					{
						name: "underlying",
						type: "address",
						internalType: "address",
					},
					{
						name: "isWeth",
						type: "bool",
						internalType: "bool",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "dispatcher",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "host",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "isWeth",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "onAccept",
		inputs: [
			{
				name: "incoming",
				type: "tuple",
				internalType: "struct IncomingPostRequest",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct PostRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "to",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "body",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onGetResponse",
		inputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct IncomingGetResponse",
				components: [
					{
						name: "response",
						type: "tuple",
						internalType: "struct GetResponse",
						components: [
							{
								name: "request",
								type: "tuple",
								internalType: "struct GetRequest",
								components: [
									{
										name: "source",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "dest",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "nonce",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "from",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "timeoutTimestamp",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "keys",
										type: "bytes[]",
										internalType: "bytes[]",
									},
									{
										name: "height",
										type: "uint64",
										internalType: "uint64",
									},
									{
										name: "context",
										type: "bytes",
										internalType: "bytes",
									},
								],
							},
							{
								name: "values",
								type: "tuple[]",
								internalType: "struct StorageValue[]",
								components: [
									{
										name: "key",
										type: "bytes",
										internalType: "bytes",
									},
									{
										name: "value",
										type: "bytes",
										internalType: "bytes",
									},
								],
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onGetTimeout",
		inputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct GetRequestTimeout",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct GetRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "keys",
								type: "bytes[]",
								internalType: "bytes[]",
							},
							{
								name: "height",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "context",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "onPostRequestTimeout",
		inputs: [
			{
				name: "incoming",
				type: "tuple",
				internalType: "struct PostRequestTimeout",
				components: [
					{
						name: "request",
						type: "tuple",
						internalType: "struct PostRequest",
						components: [
							{
								name: "source",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "dest",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "nonce",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "from",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "to",
								type: "bytes",
								internalType: "bytes",
							},
							{
								name: "timeoutTimestamp",
								type: "uint64",
								internalType: "uint64",
							},
							{
								name: "body",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "relayer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "owner",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "pause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "paused",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "request",
				type: "tuple",
				internalType: "struct DispatchPost",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "body",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "fee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "payer",
						type: "address",
						internalType: "address",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "params",
				type: "tuple",
				internalType: "struct HyperFungibleToken.SendParams",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "amount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "relayerFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "data",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "quote",
		inputs: [
			{
				name: "request",
				type: "tuple",
				internalType: "struct DispatchGet",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "height",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "keys",
						type: "bytes[]",
						internalType: "bytes[]",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "fee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "context",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "removeChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "renounceOwnership",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "send",
		inputs: [
			{
				name: "params",
				type: "tuple",
				internalType: "struct HyperFungibleToken.SendParams",
				components: [
					{
						name: "dest",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "to",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "amount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "timeout",
						type: "uint64",
						internalType: "uint64",
					},
					{
						name: "relayerFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "data",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "supportedChain",
		inputs: [
			{
				name: "chainId",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [
			{
				name: "",
				type: "bytes",
				internalType: "bytes",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "supportsInterface",
		inputs: [
			{
				name: "interfaceId",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "transferOwnership",
		inputs: [
			{
				name: "newOwner",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "underlying",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "unpause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "OwnershipTransferred",
		inputs: [
			{
				name: "previousOwner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "newOwner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Paused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Received",
		inputs: [
			{
				name: "from",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
			},
			{
				name: "to",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "source",
				type: "string",
				indexed: false,
				internalType: "string",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Refunded",
		inputs: [
			{
				name: "to",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Sent",
		inputs: [
			{
				name: "from",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "to",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
			},
			{
				name: "dest",
				type: "string",
				indexed: false,
				internalType: "string",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "commitment",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Unpaused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "error",
		name: "EnforcedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "ExpectedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidAddress",
		inputs: [
			{
				name: "length",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "OwnableInvalidOwner",
		inputs: [
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "OwnableUnauthorizedAccount",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "SafeERC20FailedOperation",
		inputs: [
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "TransferFailed",
		inputs: [],
	},
	{
		type: "error",
		name: "UnauthorizedCall",
		inputs: [],
	},
	{
		type: "error",
		name: "UnauthorizedSource",
		inputs: [],
	},
	{
		type: "error",
		name: "UnexpectedCall",
		inputs: [],
	},
	{
		type: "error",
		name: "UnsupportedChain",
		inputs: [],
	},
] as const
