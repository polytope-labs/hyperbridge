export const UNISWAP_V2_FACTORY_ABI = [
	{
		inputs: [
			{
				internalType: "address",
				name: "_feeCollectorSetter",
				type: "address",
			},
		],
		stateMutability: "nonpayable",
		type: "constructor",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				internalType: "address",
				name: "token0",
				type: "address",
			},
			{
				indexed: true,
				internalType: "address",
				name: "token1",
				type: "address",
			},
			{
				indexed: false,
				internalType: "address",
				name: "pair",
				type: "address",
			},
			{
				indexed: false,
				internalType: "uint256",
				name: "",
				type: "uint256",
			},
		],
		name: "PairCreated",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "address",
				name: "feeCollector_",
				type: "address",
			},
		],
		name: "SetFeeCollector",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "address",
				name: "feeCollectorSetter_",
				type: "address",
			},
		],
		name: "SetFeeCollectorSetter",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: false,
				internalType: "address",
				name: "migrator_",
				type: "address",
			},
		],
		name: "SetMigrator",
		type: "event",
	},
	{
		inputs: [
			{
				internalType: "uint256",
				name: "",
				type: "uint256",
			},
		],
		name: "allPairs",
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
		name: "allPairsLength",
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
				internalType: "address",
				name: "tokenA",
				type: "address",
			},
			{
				internalType: "address",
				name: "tokenB",
				type: "address",
			},
		],
		name: "createPair",
		outputs: [
			{
				internalType: "address",
				name: "pair",
				type: "address",
			},
		],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [],
		name: "feeCollector",
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
		name: "feeCollectorSetter",
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
				internalType: "address",
				name: "",
				type: "address",
			},
			{
				internalType: "address",
				name: "",
				type: "address",
			},
		],
		name: "getPair",
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
		name: "migrator",
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
		name: "pairCodeHash",
		outputs: [
			{
				internalType: "bytes32",
				name: "",
				type: "bytes32",
			},
		],
		stateMutability: "pure",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "address",
				name: "_feeCollector",
				type: "address",
			},
		],
		name: "setFeeCollector",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "address",
				name: "_feeCollectorSetter",
				type: "address",
			},
		],
		name: "setFeeCollectorSetter",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{
				internalType: "address",
				name: "_migrator",
				type: "address",
			},
		],
		name: "setMigrator",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
] as const
