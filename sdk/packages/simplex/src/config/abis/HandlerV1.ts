export const HandlerV1_ABI = [
	{
		inputs: [],
		name: "ChallengePeriodNotElapsed",
		type: "error",
	},
	{
		inputs: [],
		name: "ConsensusClientExpired",
		type: "error",
	},
	{
		inputs: [],
		name: "DuplicateMessage",
		type: "error",
	},
	{
		inputs: [],
		name: "HostFrozen",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidMessageDestination",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidProof",
		type: "error",
	},
	{
		inputs: [],
		name: "MessageNotTimedOut",
		type: "error",
	},
	{
		inputs: [],
		name: "MessageTimedOut",
		type: "error",
	},
	{
		inputs: [],
		name: "StateCommitmentNotFound",
		type: "error",
	},
	{
		inputs: [],
		name: "UnknownMessage",
		type: "error",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
			{
				internalType: "bytes",
				name: "proof",
				type: "bytes",
			},
		],
		name: "handleConsensus",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
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
						internalType: "struct GetRequest[]",
						name: "timeouts",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "uint256",
								name: "stateMachineId",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "height",
								type: "uint256",
							},
						],
						internalType: "struct StateMachineHeight",
						name: "height",
						type: "tuple",
					},
					{
						internalType: "bytes[]",
						name: "proof",
						type: "bytes[]",
					},
				],
				internalType: "struct GetTimeoutMessage",
				name: "message",
				type: "tuple",
			},
		],
		name: "handleGetRequestTimeouts",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
			{
				components: [
					{
						components: [
							{
								components: [
									{
										internalType: "uint256",
										name: "stateMachineId",
										type: "uint256",
									},
									{
										internalType: "uint256",
										name: "height",
										type: "uint256",
									},
								],
								internalType: "struct StateMachineHeight",
								name: "height",
								type: "tuple",
							},
							{
								internalType: "bytes32[]",
								name: "multiproof",
								type: "bytes32[]",
							},
							{
								internalType: "uint256",
								name: "leafCount",
								type: "uint256",
							},
						],
						internalType: "struct Proof",
						name: "proof",
						type: "tuple",
					},
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
								internalType: "uint256",
								name: "index",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "kIndex",
								type: "uint256",
							},
						],
						internalType: "struct GetResponseLeaf[]",
						name: "responses",
						type: "tuple[]",
					},
				],
				internalType: "struct GetResponseMessage",
				name: "message",
				type: "tuple",
			},
		],
		name: "handleGetResponses",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
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
						internalType: "struct PostRequest[]",
						name: "timeouts",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "uint256",
								name: "stateMachineId",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "height",
								type: "uint256",
							},
						],
						internalType: "struct StateMachineHeight",
						name: "height",
						type: "tuple",
					},
					{
						internalType: "bytes[]",
						name: "proof",
						type: "bytes[]",
					},
				],
				internalType: "struct PostRequestTimeoutMessage",
				name: "message",
				type: "tuple",
			},
		],
		name: "handlePostRequestTimeouts",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
			{
				components: [
					{
						components: [
							{
								components: [
									{
										internalType: "uint256",
										name: "stateMachineId",
										type: "uint256",
									},
									{
										internalType: "uint256",
										name: "height",
										type: "uint256",
									},
								],
								internalType: "struct StateMachineHeight",
								name: "height",
								type: "tuple",
							},
							{
								internalType: "bytes32[]",
								name: "multiproof",
								type: "bytes32[]",
							},
							{
								internalType: "uint256",
								name: "leafCount",
								type: "uint256",
							},
						],
						internalType: "struct Proof",
						name: "proof",
						type: "tuple",
					},
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
								internalType: "uint256",
								name: "index",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "kIndex",
								type: "uint256",
							},
						],
						internalType: "struct PostRequestLeaf[]",
						name: "requests",
						type: "tuple[]",
					},
				],
				internalType: "struct PostRequestMessage",
				name: "request",
				type: "tuple",
			},
		],
		name: "handlePostRequests",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
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
						internalType: "struct PostResponse[]",
						name: "timeouts",
						type: "tuple[]",
					},
					{
						components: [
							{
								internalType: "uint256",
								name: "stateMachineId",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "height",
								type: "uint256",
							},
						],
						internalType: "struct StateMachineHeight",
						name: "height",
						type: "tuple",
					},
					{
						internalType: "bytes[]",
						name: "proof",
						type: "bytes[]",
					},
				],
				internalType: "struct PostResponseTimeoutMessage",
				name: "message",
				type: "tuple",
			},
		],
		name: "handlePostResponseTimeouts",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "contract IIsmpHost",
				name: "host",
				type: "address",
			},
			{
				components: [
					{
						components: [
							{
								components: [
									{
										internalType: "uint256",
										name: "stateMachineId",
										type: "uint256",
									},
									{
										internalType: "uint256",
										name: "height",
										type: "uint256",
									},
								],
								internalType: "struct StateMachineHeight",
								name: "height",
								type: "tuple",
							},
							{
								internalType: "bytes32[]",
								name: "multiproof",
								type: "bytes32[]",
							},
							{
								internalType: "uint256",
								name: "leafCount",
								type: "uint256",
							},
						],
						internalType: "struct Proof",
						name: "proof",
						type: "tuple",
					},
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
								internalType: "uint256",
								name: "index",
								type: "uint256",
							},
							{
								internalType: "uint256",
								name: "kIndex",
								type: "uint256",
							},
						],
						internalType: "struct PostResponseLeaf[]",
						name: "responses",
						type: "tuple[]",
					},
				],
				internalType: "struct PostResponseMessage",
				name: "response",
				type: "tuple",
			},
		],
		name: "handlePostResponses",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "bytes4",
				name: "interfaceId",
				type: "bytes4",
			},
		],
		name: "supportsInterface",
		outputs: [
			{
				internalType: "bool",
				name: "",
				type: "bool",
			},
		],
		stateMutability: "view",
		type: "function",
	},
] as const
