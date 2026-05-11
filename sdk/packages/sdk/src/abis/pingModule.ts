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
		name: "ExecutionFailed",
		type: "error",
	},
	{
		inputs: [],
		name: "NotIsmpHost",
		type: "error",
	},
	{
		anonymous: false,
		inputs: [
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
				indexed: false,
				internalType: "struct StorageValue[]",
				name: "message",
				type: "tuple[]",
			},
		],
		name: "GetResponseReceived",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [],
		name: "GetTimeoutReceived",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [],
		name: "MessageDispatched",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "message",
				type: "string",
			},
		],
		name: "PostReceived",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [],
		name: "PostRequestTimeoutReceived",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [],
		name: "PostResponseReceived",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [],
		name: "PostResponseTimeoutReceived",
		type: "event",
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
				name: "request",
				type: "tuple",
			},
		],
		name: "dispatch",
		outputs: [
			{
				internalType: "bytes32",
				name: "",
				type: "bytes32",
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
		],
		name: "dispatchPostResponse",
		outputs: [
			{
				internalType: "bytes32",
				name: "",
				type: "bytes32",
			},
		],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "uint256",
				name: "_paraId",
				type: "uint256",
			},
		],
		name: "dispatchToParachain",
		outputs: [],
		stateMutability: "nonpayable",
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
				name: "response",
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
		inputs: [
			{
				components: [
					{
						internalType: "bytes",
						name: "dest",
						type: "bytes",
					},
					{
						internalType: "address",
						name: "module",
						type: "address",
					},
					{
						internalType: "uint64",
						name: "timeout",
						type: "uint64",
					},
					{
						internalType: "uint256",
						name: "count",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
				],
				internalType: "struct PingMessage",
				name: "pingMessage",
				type: "tuple",
			},
		],
		name: "ping",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "previousPostRequest",
		outputs: [
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
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "address",
				name: "hostAddr",
				type: "address",
			},
			{
				internalType: "address",
				name: "tokenFaucet",
				type: "address",
			},
		],
		name: "setIsmpHost",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
] as const

export default { ABI }
