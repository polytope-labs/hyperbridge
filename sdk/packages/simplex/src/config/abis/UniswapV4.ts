/**
 * Uniswap V4 ABI definitions for PositionManager, PoolManager, and StateView.
 *
 * PositionManager manages concentrated-liquidity positions as ERC-721 NFTs.
 * PoolManager stores all pool state (singleton).
 * StateView provides read-only access to pool state (slot0, liquidity, ticks).
 */

// ---------------------------------------------------------------------------
// Shared components
// ---------------------------------------------------------------------------

/**
 * PoolKey is the canonical identifier for a V4 pool.
 * Shared tuple definition reused across multiple ABI entries.
 */
const POOL_KEY_COMPONENTS = [
	{ name: "currency0", type: "address" },
	{ name: "currency1", type: "address" },
	{ name: "fee", type: "uint24" },
	{ name: "tickSpacing", type: "int24" },
	{ name: "hooks", type: "address" },
] as const

// ---------------------------------------------------------------------------
// PositionManager ABI
// ---------------------------------------------------------------------------

export const UNISWAP_V4_POSITION_MANAGER_ABI = [
	{
		name: "getPoolAndPositionInfo",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "tokenId", type: "uint256" }],
		outputs: [
			{
				name: "poolKey",
				type: "tuple",
				components: POOL_KEY_COMPONENTS,
			},
			// PositionInfo is a packed uint256:
			// 200 bits poolId | 24 bits tickUpper | 24 bits tickLower | 8 bits hasSubscriber
			{ name: "info", type: "uint256" },
		],
	},
	{
		name: "modifyLiquidities",
		type: "function",
		stateMutability: "payable",
		inputs: [
			{ name: "unlockData", type: "bytes" },
			{ name: "deadline", type: "uint256" },
		],
		outputs: [],
	},
	{
		name: "multicall",
		type: "function",
		stateMutability: "payable",
		inputs: [{ name: "data", type: "bytes[]" }],
		outputs: [{ name: "results", type: "bytes[]" }],
	},
	{
		name: "ownerOf",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "tokenId", type: "uint256" }],
		outputs: [{ type: "address" }],
	},
	{
		name: "getPositionLiquidity",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "tokenId", type: "uint256" }],
		outputs: [{ name: "liquidity", type: "uint128" }],
	},
] as const

// ---------------------------------------------------------------------------
// PoolManager ABI (IExtsload) — retained for any direct storage reads
// ---------------------------------------------------------------------------

export const UNISWAP_V4_POOL_MANAGER_ABI = [
	{
		name: "extsload",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "slot", type: "bytes32" }],
		outputs: [{ name: "value", type: "bytes32" }],
	},
] as const

// ---------------------------------------------------------------------------
// StateView ABI — used for clean pool state reads (replaces raw extsload)
// ---------------------------------------------------------------------------

export const UNISWAP_V4_STATE_VIEW_ABI = [
	{
		name: "getSlot0",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "poolId", type: "bytes32" }],
		outputs: [
			{ name: "sqrtPriceX96", type: "uint160" },
			{ name: "tick", type: "int24" },
			{ name: "protocolFee", type: "uint24" },
			{ name: "lpFee", type: "uint24" },
		],
	},
	{
		name: "getLiquidity",
		type: "function",
		stateMutability: "view",
		inputs: [{ name: "poolId", type: "bytes32" }],
		outputs: [{ name: "liquidity", type: "uint128" }],
	},
	{
		name: "getTickInfo",
		type: "function",
		stateMutability: "view",
		inputs: [
			{ name: "poolId", type: "bytes32" },
			{ name: "tick", type: "int24" },
		],
		outputs: [
			{ name: "liquidityGross", type: "uint128" },
			{ name: "liquidityNet", type: "int128" },
			{ name: "feeGrowthOutside0X128", type: "uint256" },
			{ name: "feeGrowthOutside1X128", type: "uint256" },
		],
	},
	{
		name: "getTickBitmap",
		type: "function",
		stateMutability: "view",
		inputs: [
			{ name: "poolId", type: "bytes32" },
			{ name: "wordPosition", type: "int16" },
		],
		outputs: [{ name: "tickBitmap", type: "uint256" }],
	},
	{
		name: "getFeeGrowthInside",
		type: "function",
		stateMutability: "view",
		inputs: [
			{ name: "poolId", type: "bytes32" },
			{ name: "tickLower", type: "int24" },
			{ name: "tickUpper", type: "int24" },
		],
		outputs: [
			{ name: "feeGrowthInside0X128", type: "uint256" },
			{ name: "feeGrowthInside1X128", type: "uint256" },
		],
	},
	{
		name: "getPositionInfo",
		type: "function",
		stateMutability: "view",
		inputs: [
			{
				name: "poolKey",
				type: "tuple",
				components: POOL_KEY_COMPONENTS,
			},
			{ name: "owner", type: "address" },
			{ name: "tickLower", type: "int24" },
			{ name: "tickUpper", type: "int24" },
			{ name: "salt", type: "bytes32" },
		],
		outputs: [
			{ name: "liquidity", type: "uint128" },
			{ name: "feeGrowthInside0LastX128", type: "uint256" },
			{ name: "feeGrowthInside1LastX128", type: "uint256" },
		],
	},
] as const

// ---------------------------------------------------------------------------
// Action constants for modifyLiquidities unlockData encoding
// (Retained for any manual encoding paths; the v4-sdk handles these
// internally via V4PositionManager / V4Planner)
// ---------------------------------------------------------------------------

/** Action opcodes from v4-periphery/src/libraries/Actions.sol */
export const V4_ACTIONS = {
	INCREASE_LIQUIDITY: 0x00,
	DECREASE_LIQUIDITY: 0x01,
	MINT_POSITION: 0x02,
	BURN_POSITION: 0x03,
	SWAP_EXACT_IN_SINGLE: 0x04,
	SWAP_EXACT_IN: 0x05,
	SWAP_EXACT_OUT_SINGLE: 0x06,
	SWAP_EXACT_OUT: 0x07,
	DONATE: 0x08,
	SETTLE: 0x09,
	SETTLE_ALL: 0x0a,
	SETTLE_PAIR: 0x0b,
	TAKE: 0x0c,
	TAKE_ALL: 0x0d,
	TAKE_PAIR: 0x0e,
	TAKE_PORTION: 0x0f,
	CLOSE_CURRENCY: 0x10,
	CLEAR_OR_TAKE: 0x11,
	SWEEP: 0x12,
} as const

// ---------------------------------------------------------------------------
// PositionInfo (packed uint256) decoding helpers
// ---------------------------------------------------------------------------

/**
 * Extracts tickLower from a packed PositionInfo uint256.
 * Layout: ... | 24 bits tickUpper | 24 bits tickLower | 8 bits hasSubscriber
 */
export function decodeTickLower(positionInfo: bigint): number {
	const raw = Number((positionInfo >> 8n) & 0xffffffn)
	return raw >= 0x800000 ? raw - 0x1000000 : raw
}

/**
 * Extracts tickUpper from a packed PositionInfo uint256.
 * Layout: ... | 24 bits tickUpper | 24 bits tickLower | 8 bits hasSubscriber
 */
export function decodeTickUpper(positionInfo: bigint): number {
	const raw = Number((positionInfo >> 32n) & 0xffffffn)
	return raw >= 0x800000 ? raw - 0x1000000 : raw
}

// ---------------------------------------------------------------------------
// Slot0 (packed bytes32) decoding helpers — retained for any direct reads
// ---------------------------------------------------------------------------

/**
 * Extracts sqrtPriceX96 (lowest 160 bits) from a packed Slot0 bytes32.
 * Layout: 24 lpFee | 12 protocolFee(1→0) | 12 protocolFee(0→1) | 24 tick | 160 sqrtPriceX96
 */
export function decodeSlot0SqrtPriceX96(slot0: bigint): bigint {
	return slot0 & ((1n << 160n) - 1n)
}

/**
 * Extracts tick (bits 160-183) from a packed Slot0 bytes32.
 */
export function decodeSlot0Tick(slot0: bigint): number {
	const raw = Number((slot0 >> 160n) & 0xffffffn)
	return raw >= 0x800000 ? raw - 0x1000000 : raw
}

// ---------------------------------------------------------------------------
// PoolManager storage slot — retained for any direct extsload paths
// ---------------------------------------------------------------------------

/**
 * The `_pools` mapping storage slot in PoolManager.
 * `mapping(PoolId id => Pool.State) internal _pools` is at slot 6
 * in the canonical v4-core PoolManager layout.
 */
export const POOLS_MAPPING_SLOT = 6n
