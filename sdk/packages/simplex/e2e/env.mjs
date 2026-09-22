// Settings for the testnet swap end-to-end run. Every key and every endpoint comes from the
// environment; only public contract addresses live here.
import { bscTestnet, polygonAmoy } from "viem/chains"

/** Reads a required variable, naming every missing one at once. */
export function readEnv() {
	const names = {
		bscRpc: "E2E_BSC_TESTNET_RPC_URL",
		amoyRpc: "E2E_POLYGON_AMOY_RPC_URL",
		orderbook: "E2E_ORDERBOOK_URL",
		hyperbridge: "E2E_HYPERBRIDGE_WS_URL",
		solver1Key: "E2E_SOLVER1_PRIVATE_KEY",
		solver2Key: "E2E_SOLVER2_PRIVATE_KEY",
		solver3Key: "E2E_SOLVER3_PRIVATE_KEY",
		solver1Substrate: "E2E_SOLVER1_SUBSTRATE_KEY",
		solver2Substrate: "E2E_SOLVER2_SUBSTRATE_KEY",
		solver3Substrate: "E2E_SOLVER3_SUBSTRATE_KEY",
		user1Key: "E2E_USER1_PRIVATE_KEY",
		user2Key: "E2E_USER2_PRIVATE_KEY",
	}
	const missing = Object.values(names).filter((name) => !process.env[name])
	if (missing.length > 0) throw new Error(`Missing environment variables: ${missing.join(", ")}`)
	const env = Object.fromEntries(Object.entries(names).map(([key, name]) => [key, process.env[name]]))
	// A bundler is optional: Alchemy serves ERC-4337 on the same endpoint as the RPC.
	env.bscBundler = process.env.E2E_BSC_TESTNET_BUNDLER_URL || env.bscRpc
	env.amoyBundler = process.env.E2E_POLYGON_AMOY_BUNDLER_URL || env.amoyRpc
	return env
}

export const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA"
export const HOST = "0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1"
export const FEE_TOKEN = "0xBE97E73126D66188d72fbF99029126D0340a7f18"

export const TOKENS = {
	"EVM-97": {
		USDC: { address: "0xA801da100bF16D07F668F4A49E1f71fc54D05177", decimals: 18 },
		cNGN: { address: "0x2bbbd701cfC25D37f18127e51Df0933566D5778a", decimals: 6 },
	},
	"EVM-80002": {
		USDC: { address: "0xBE97E73126D66188d72fbF99029126D0340a7f18", decimals: 18 },
		cNGN: { address: "0xE4ff5d2AE65C10f530C000215029919961E8A758", decimals: 6 },
	},
}

export function chains(env) {
	return {
		"EVM-97": { id: 97, viem: bscTestnet, rpc: env.bscRpc, bundler: env.bscBundler },
		"EVM-80002": { id: 80002, viem: polygonAmoy, rpc: env.amoyRpc, bundler: env.amoyBundler },
	}
}

/**
 * The three solvers and the prices of their standing limit orders, in cNGN per USDC:
 *  - A buys cNGN for USDC, filled on Polygon Amoy
 *  - B buys cNGN for USDC, filled on BSC Chapel
 *  - C buys USDC for cNGN, filled on BSC Chapel
 * Solver 1 is always the best priced, so scenarios can target its levels alone.
 */
export const SOLVERS = [
	{ name: "solver1", port: 8701, a: 1580, b: 1578, c: 1600 },
	{ name: "solver2", port: 8702, a: 1570, b: 1568, c: 1610 },
	{ name: "solver3", port: 8703, a: 1560, b: 1558, c: 1620 },
]

/** Human decimal string with at most 12 fractional digits. */
const dec = (x) => x.toFixed(12).replace(/\.?0+$/, "")

export function standingOrders(solver) {
	const sources = ["EVM-97", "EVM-80002"]
	return [
		{ fillChain: "EVM-80002", tokenIn: "USDC", amountIn: "200", tokenOut: "cNGN", amountOut: dec(200 * solver.a), acceptedSources: sources },
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "200", tokenOut: "cNGN", amountOut: dec(200 * solver.b), acceptedSources: sources },
		{ fillChain: "EVM-97", tokenIn: "cNGN", amountIn: "300000", tokenOut: "USDC", amountOut: dec(300000 / solver.c), acceptedSources: sources },
	]
}

/** Extra solver 1 levels on BSC Chapel, above every standing order. */
export const EXTRA_LEVELS = {
	// Buy cNGN at 1590 and 1585.
	bids: [
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "25", tokenOut: "cNGN", amountOut: "39750", acceptedSources: ["EVM-97", "EVM-80002"] },
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "20", tokenOut: "cNGN", amountOut: "31700", acceptedSources: ["EVM-97", "EVM-80002"] },
	],
	// Sell cNGN at 1590.
	ask: [{ fillChain: "EVM-97", tokenIn: "cNGN", amountIn: "47700", tokenOut: "USDC", amountOut: "30", acceptedSources: ["EVM-97", "EVM-80002"] }],
}

/**
 * Each scenario runs against a freshly posted book: the standing orders, plus the extra
 * solver 1 levels it names. `minFills` is the fewest fill transactions it must take.
 */
export const SCENARIOS = {
	"same-chain": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		legs: [{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "77850" }],
		minFills: 1,
	},
	"cross-chain": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-80002",
		legs: [{ tokenIn: "USDC", amountIn: "40", tokenOut: "cNGN", minOut: "62200" }],
		minFills: 1,
	},
	// 500000 cNGN is more than any one solver's 300000 cNGN order takes, so it fills in parts.
	partial: {
		user: 1,
		source: "EVM-80002",
		dest: "EVM-97",
		legs: [{ tokenIn: "cNGN", amountIn: "500000", tokenOut: "USDC", minOut: "305.5" }],
		minFills: 2,
	},
	// Legs on two pairs, each reaching a different set of limit orders: leg 0 at 1565+ cNGN per
	// USDC (solver 1 and 2), leg 1 at up to ~1625 (all three).
	"multi-leg": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		legs: [
			{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "78210" },
			{ tokenIn: "cNGN", amountIn: "100000", tokenOut: "USDC", minOut: "61.5" },
		],
		minFills: 2,
	},
	// Only solver 1 clears 1574 cNGN per USDC, and its two extra levels cannot cover it alone:
	// several bids from one solver, best rate first.
	"same-solver-levels": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids"],
		legs: [{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "78710" }],
		minFills: 2,
	},
	// Both pairs, solver 1's levels only on each side.
	"multi-leg-levels": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids", "ask"],
		legs: [
			{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "78710" },
			{ tokenIn: "cNGN", amountIn: "100000", tokenOut: "USDC", minOut: "62.27" },
		],
		minFills: 2,
	},
	// One pair at two legs, both served by solver 1's levels.
	"multi-leg-same-input": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids"],
		legs: [
			{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "78710" },
			{ tokenIn: "USDC", amountIn: "50", tokenOut: "cNGN", minOut: "78710" },
		],
		minFills: 2,
	},
}
