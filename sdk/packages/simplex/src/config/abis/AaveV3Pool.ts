/**
 * Minimal Aave V3 `Pool` ABI.
 *
 * Only the two methods the funding venue needs:
 *   - `withdraw` to source supplied liquidity (burns aTokens, sends underlying)
 *   - `getReserveData` to resolve an asset's aToken address once at startup
 *
 * `getReserveData` returns the `ReserveDataLegacy` struct, which is stable
 * across Aave V3.0–3.x (the stable-rate fields are retained for compatibility).
 * We only read `aTokenAddress`.
 */
export const AAVE_V3_POOL_ABI = [
	{
		inputs: [
			{ internalType: "address", name: "asset", type: "address" },
			{ internalType: "uint256", name: "amount", type: "uint256" },
			{ internalType: "address", name: "to", type: "address" },
		],
		name: "withdraw",
		outputs: [{ internalType: "uint256", name: "", type: "uint256" }],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [
			{ internalType: "address", name: "asset", type: "address" },
			{ internalType: "uint256", name: "amount", type: "uint256" },
			{ internalType: "address", name: "onBehalfOf", type: "address" },
			{ internalType: "uint16", name: "referralCode", type: "uint16" },
		],
		name: "supply",
		outputs: [],
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		inputs: [{ internalType: "address", name: "asset", type: "address" }],
		name: "getReserveData",
		outputs: [
			{
				components: [
					{
						components: [{ internalType: "uint256", name: "data", type: "uint256" }],
						internalType: "struct DataTypes.ReserveConfigurationMap",
						name: "configuration",
						type: "tuple",
					},
					{ internalType: "uint128", name: "liquidityIndex", type: "uint128" },
					{ internalType: "uint128", name: "currentLiquidityRate", type: "uint128" },
					{ internalType: "uint128", name: "variableBorrowIndex", type: "uint128" },
					{ internalType: "uint128", name: "currentVariableBorrowRate", type: "uint128" },
					{ internalType: "uint128", name: "currentStableBorrowRate", type: "uint128" },
					{ internalType: "uint40", name: "lastUpdateTimestamp", type: "uint40" },
					{ internalType: "uint16", name: "id", type: "uint16" },
					{ internalType: "address", name: "aTokenAddress", type: "address" },
					{ internalType: "address", name: "stableDebtTokenAddress", type: "address" },
					{ internalType: "address", name: "variableDebtTokenAddress", type: "address" },
					{ internalType: "address", name: "interestRateStrategyAddress", type: "address" },
					{ internalType: "uint128", name: "accruedToTreasury", type: "uint128" },
					{ internalType: "uint128", name: "unbacked", type: "uint128" },
					{ internalType: "uint128", name: "isolationModeTotalDebt", type: "uint128" },
				],
				internalType: "struct DataTypes.ReserveDataLegacy",
				name: "",
				type: "tuple",
			},
		],
		stateMutability: "view",
		type: "function",
	},
] as const
