export const EVM_HOST = [
	{
		inputs: [],
		name: "CannotChangeFeeToken",
		type: "error",
	},
	{
		inputs: [],
		name: "DuplicateResponse",
		type: "error",
	},
	{
		inputs: [],
		name: "FrozenHost",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidAddressLength",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidConsensusClient",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidHandler",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidHostManager",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidHyperbridgeId",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidStateMachinesLength",
		type: "error",
	},
	{
		inputs: [],
		name: "InvalidUnstakingPeriod",
		type: "error",
	},
	{
		inputs: [],
		name: "MaxFishermanCountExceeded",
		type: "error",
	},
	{
		inputs: [],
		name: "UnauthorizedAccount",
		type: "error",
	},
	{
		inputs: [],
		name: "UnauthorizedAction",
		type: "error",
	},
	{
		inputs: [],
		name: "UnauthorizedResponse",
		type: "error",
	},
	{
		inputs: [],
		name: "UnknownRequest",
		type: "error",
	},
	{
		inputs: [],
		name: "UnknownResponse",
		type: "error",
	},
	{
		inputs: [],
		name: "WithdrawalFailed",
		type: "error",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "source",
				type: "string",
			},
			{
				indexed: false,
				internalType: "string",
				name: "dest",
				type: "string",
			},
			{
				indexed: true,
				internalType: "address",
				name: "from",
				type: "address",
			},
			{
				indexed: false,
				internalType: "bytes[]",
				name: "keys",
				type: "bytes[]",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "height",
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
				name: "timeoutTimestamp",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "context",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "fee",
				type: "uint256",
			},
		],
		name: "GetRequestEvent",
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
				name: "relayer",
				type: "address",
			},
		],
		name: "GetRequestHandled",
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
				internalType: "string",
				name: "dest",
				type: "string",
			},
		],
		name: "GetRequestTimeoutHandled",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "enum FrozenStatus",
				name: "status",
				type: "uint8",
			},
		],
		name: "HostFrozen",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				components: [
					{
						internalType: "uint256",
						name: "defaultTimeout",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "perByteFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "stateCommitmentFee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "feeToken",
						type: "address",
					},
					{
						internalType: "address",
						name: "admin",
						type: "address",
					},
					{
						internalType: "address",
						name: "handler",
						type: "address",
					},
					{
						internalType: "address",
						name: "hostManager",
						type: "address",
					},
					{
						internalType: "address",
						name: "uniswapV2",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "unStakingPeriod",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "challengePeriod",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "consensusClient",
						type: "address",
					},
					{
						internalType: "uint256[]",
						name: "stateMachines",
						type: "uint256[]",
					},
					{
						internalType: "address[]",
						name: "fishermen",
						type: "address[]",
					},
					{
						internalType: "bytes",
						name: "hyperbridge",
						type: "bytes",
					},
				],
				indexed: false,
				internalType: "struct HostParams",
				name: "oldParams",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "defaultTimeout",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "perByteFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "stateCommitmentFee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "feeToken",
						type: "address",
					},
					{
						internalType: "address",
						name: "admin",
						type: "address",
					},
					{
						internalType: "address",
						name: "handler",
						type: "address",
					},
					{
						internalType: "address",
						name: "hostManager",
						type: "address",
					},
					{
						internalType: "address",
						name: "uniswapV2",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "unStakingPeriod",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "challengePeriod",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "consensusClient",
						type: "address",
					},
					{
						internalType: "uint256[]",
						name: "stateMachines",
						type: "uint256[]",
					},
					{
						internalType: "address[]",
						name: "fishermen",
						type: "address[]",
					},
					{
						internalType: "bytes",
						name: "hyperbridge",
						type: "bytes",
					},
				],
				indexed: false,
				internalType: "struct HostParams",
				name: "newParams",
				type: "tuple",
			},
		],
		name: "HostParamsUpdated",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
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
			{
				indexed: false,
				internalType: "bool",
				name: "native",
				type: "bool",
			},
		],
		name: "HostWithdrawal",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "source",
				type: "string",
			},
			{
				indexed: false,
				internalType: "string",
				name: "dest",
				type: "string",
			},
			{
				indexed: true,
				internalType: "address",
				name: "from",
				type: "address",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "to",
				type: "bytes",
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
				name: "timeoutTimestamp",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "body",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "fee",
				type: "uint256",
			},
		],
		name: "PostRequestEvent",
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
				name: "relayer",
				type: "address",
			},
		],
		name: "PostRequestHandled",
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
				internalType: "string",
				name: "dest",
				type: "string",
			},
		],
		name: "PostRequestTimeoutHandled",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "source",
				type: "string",
			},
			{
				indexed: false,
				internalType: "string",
				name: "dest",
				type: "string",
			},
			{
				indexed: true,
				internalType: "address",
				name: "from",
				type: "address",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "to",
				type: "bytes",
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
				name: "timeoutTimestamp",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "body",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "bytes",
				name: "response",
				type: "bytes",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "responseTimeoutTimestamp",
				type: "uint256",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "fee",
				type: "uint256",
			},
		],
		name: "PostResponseEvent",
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
				internalType: "uint256",
				name: "newFee",
				type: "uint256",
			},
		],
		name: "PostResponseFunded",
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
				name: "relayer",
				type: "address",
			},
		],
		name: "PostResponseHandled",
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
				internalType: "string",
				name: "dest",
				type: "string",
			},
		],
		name: "PostResponseTimeoutHandled",
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
				internalType: "uint256",
				name: "newFee",
				type: "uint256",
			},
		],
		name: "RequestFunded",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				internalType: "address",
				name: "caller",
				type: "address",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "fee",
				type: "uint256",
			},
		],
		name: "StateCommitmentRead",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "stateMachineId",
				type: "string",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "height",
				type: "uint256",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "timestamp",
						type: "uint256",
					},
					{
						internalType: "bytes32",
						name: "overlayRoot",
						type: "bytes32",
					},
					{
						internalType: "bytes32",
						name: "stateRoot",
						type: "bytes32",
					},
				],
				indexed: false,
				internalType: "struct StateCommitment",
				name: "stateCommitment",
				type: "tuple",
			},
			{
				indexed: true,
				internalType: "address",
				name: "fisherman",
				type: "address",
			},
		],
		name: "StateCommitmentVetoed",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "string",
				name: "stateMachineId",
				type: "string",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "height",
				type: "uint256",
			},
		],
		name: "StateMachineUpdated",
		type: "event",
	},
	{
		inputs: [],
		name: "admin",
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
		name: "chainId",
		outputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
			},
		],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "challengePeriod",
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
		inputs: [],
		name: "consensusClient",
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
		name: "consensusState",
		outputs: [
			{
				internalType: "bytes",
				name: "",
				type: "bytes",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "consensusUpdateTime",
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
				internalType: "address",
				name: "fisherman",
				type: "address",
			},
		],
		name: "deleteStateMachineCommitment",
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
				name: "post",
				type: "tuple",
			},
		],
		name: "dispatch",
		outputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
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
				name: "post",
				type: "tuple",
			},
		],
		name: "dispatch",
		outputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
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
				name: "get",
				type: "tuple",
			},
		],
		name: "dispatch",
		outputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		stateMutability: "payable",
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
			{
				internalType: "address",
				name: "relayer",
				type: "address",
			},
		],
		name: "dispatchIncoming",
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
				name: "request",
				type: "tuple",
			},
			{
				internalType: "address",
				name: "relayer",
				type: "address",
			},
		],
		name: "dispatchIncoming",
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
		name: "dispatchIncoming",
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
				name: "response",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "sender",
						type: "address",
					},
				],
				internalType: "struct FeeMetadata",
				name: "meta",
				type: "tuple",
			},
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "dispatchTimeOut",
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
				name: "request",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "sender",
						type: "address",
					},
				],
				internalType: "struct FeeMetadata",
				name: "meta",
				type: "tuple",
			},
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "dispatchTimeOut",
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
				name: "request",
				type: "tuple",
			},
			{
				components: [
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "sender",
						type: "address",
					},
				],
				internalType: "struct FeeMetadata",
				name: "meta",
				type: "tuple",
			},
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "dispatchTimeOut",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "feeToken",
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
		name: "frozen",
		outputs: [
			{
				internalType: "enum FrozenStatus",
				name: "",
				type: "uint8",
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
			{
				internalType: "uint256",
				name: "amount",
				type: "uint256",
			},
		],
		name: "fundRequest",
		outputs: [],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
			{
				internalType: "uint256",
				name: "amount",
				type: "uint256",
			},
		],
		name: "fundResponse",
		outputs: [],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [],
		name: "host",
		outputs: [
			{
				internalType: "bytes",
				name: "",
				type: "bytes",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "hostParams",
		outputs: [
			{
				components: [
					{
						internalType: "uint256",
						name: "defaultTimeout",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "perByteFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "stateCommitmentFee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "feeToken",
						type: "address",
					},
					{
						internalType: "address",
						name: "admin",
						type: "address",
					},
					{
						internalType: "address",
						name: "handler",
						type: "address",
					},
					{
						internalType: "address",
						name: "hostManager",
						type: "address",
					},
					{
						internalType: "address",
						name: "uniswapV2",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "unStakingPeriod",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "challengePeriod",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "consensusClient",
						type: "address",
					},
					{
						internalType: "uint256[]",
						name: "stateMachines",
						type: "uint256[]",
					},
					{
						internalType: "address[]",
						name: "fishermen",
						type: "address[]",
					},
					{
						internalType: "bytes",
						name: "hyperbridge",
						type: "bytes",
					},
				],
				internalType: "struct HostParams",
				name: "",
				type: "tuple",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [],
		name: "hyperbridge",
		outputs: [
			{
				internalType: "bytes",
				name: "",
				type: "bytes",
			},
		],
		stateMutability: "view",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "uint256",
				name: "id",
				type: "uint256",
			},
		],
		name: "latestStateMachineHeight",
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
		inputs: [],
		name: "nonce",
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
		inputs: [],
		name: "perByteFee",
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
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "requestCommitments",
		outputs: [
			{
				components: [
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "sender",
						type: "address",
					},
				],
				internalType: "struct FeeMetadata",
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
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "requestReceipts",
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
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "responded",
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
	{
		inputs: [
			{
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "responseCommitments",
		outputs: [
			{
				components: [
					{
						internalType: "uint256",
						name: "fee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "sender",
						type: "address",
					},
				],
				internalType: "struct FeeMetadata",
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
				internalType: "bytes32",
				name: "commitment",
				type: "bytes32",
			},
		],
		name: "responseReceipts",
		outputs: [
			{
				components: [
					{
						internalType: "bytes32",
						name: "responseCommitment",
						type: "bytes32",
					},
					{
						internalType: "address",
						name: "relayer",
						type: "address",
					},
				],
				internalType: "struct ResponseReceipt",
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
				internalType: "bytes",
				name: "state",
				type: "bytes",
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
				components: [
					{
						internalType: "uint256",
						name: "timestamp",
						type: "uint256",
					},
					{
						internalType: "bytes32",
						name: "overlayRoot",
						type: "bytes32",
					},
					{
						internalType: "bytes32",
						name: "stateRoot",
						type: "bytes32",
					},
				],
				internalType: "struct StateCommitment",
				name: "commitment",
				type: "tuple",
			},
		],
		name: "setConsensusState",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "enum FrozenStatus",
				name: "newState",
				type: "uint8",
			},
		],
		name: "setFrozenState",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "stateCommitmentFee",
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
		],
		name: "stateMachineCommitment",
		outputs: [
			{
				components: [
					{
						internalType: "uint256",
						name: "timestamp",
						type: "uint256",
					},
					{
						internalType: "bytes32",
						name: "overlayRoot",
						type: "bytes32",
					},
					{
						internalType: "bytes32",
						name: "stateRoot",
						type: "bytes32",
					},
				],
				internalType: "struct StateCommitment",
				name: "",
				type: "tuple",
			},
		],
		stateMutability: "payable",
		type: "function",
	},
	{
		inputs: [
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
		],
		name: "stateMachineCommitmentUpdateTime",
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
				internalType: "bytes",
				name: "parachainId",
				type: "bytes",
			},
			{
				internalType: "uint256",
				name: "id",
				type: "uint256",
			},
		],
		name: "stateMachineId",
		outputs: [
			{
				internalType: "string",
				name: "",
				type: "string",
			},
		],
		stateMutability: "pure",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "bytes",
				name: "state",
				type: "bytes",
			},
		],
		name: "storeConsensusState",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
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
				components: [
					{
						internalType: "uint256",
						name: "timestamp",
						type: "uint256",
					},
					{
						internalType: "bytes32",
						name: "overlayRoot",
						type: "bytes32",
					},
					{
						internalType: "bytes32",
						name: "stateRoot",
						type: "bytes32",
					},
				],
				internalType: "struct StateCommitment",
				name: "commitment",
				type: "tuple",
			},
		],
		name: "storeStateMachineCommitment",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "timestamp",
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
		inputs: [],
		name: "unStakingPeriod",
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
		inputs: [],
		name: "uniswapV2Router",
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
						internalType: "uint256",
						name: "defaultTimeout",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "perByteFee",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "stateCommitmentFee",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "feeToken",
						type: "address",
					},
					{
						internalType: "address",
						name: "admin",
						type: "address",
					},
					{
						internalType: "address",
						name: "handler",
						type: "address",
					},
					{
						internalType: "address",
						name: "hostManager",
						type: "address",
					},
					{
						internalType: "address",
						name: "uniswapV2",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "unStakingPeriod",
						type: "uint256",
					},
					{
						internalType: "uint256",
						name: "challengePeriod",
						type: "uint256",
					},
					{
						internalType: "address",
						name: "consensusClient",
						type: "address",
					},
					{
						internalType: "uint256[]",
						name: "stateMachines",
						type: "uint256[]",
					},
					{
						internalType: "address[]",
						name: "fishermen",
						type: "address[]",
					},
					{
						internalType: "bytes",
						name: "hyperbridge",
						type: "bytes",
					},
				],
				internalType: "struct HostParams",
				name: "params",
				type: "tuple",
			},
		],
		name: "updateHostParams",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
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
		],
		name: "vetoStateCommitment",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "uint256",
				name: "paraId",
				type: "uint256",
			},
			{
				internalType: "uint256",
				name: "height",
				type: "uint256",
			},
		],
		name: "vetoes",
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
						internalType: "address",
						name: "beneficiary",
						type: "address",
					},
					{
						internalType: "uint256",
						name: "amount",
						type: "uint256",
					},
					{
						internalType: "bool",
						name: "native",
						type: "bool",
					},
				],
				internalType: "struct WithdrawParams",
				name: "params",
				type: "tuple",
			},
		],
		name: "withdraw",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		stateMutability: "payable",
		type: "receive",
	},
] as const
