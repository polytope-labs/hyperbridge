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
	// Simplex posts to the orderbook URL as given; a deployment's base URL serves GraphQL under it.
	const orderbook = env.orderbook.replace(/\/+$/, "")
	env.orderbook = orderbook.endsWith("/graphql") ? orderbook : `${orderbook}/graphql`
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

/**
 * The orderbook refuses a limit order paying out under 10 USDC or 15000 cNGN
 * (`serverInfo.minOrderSizes`), so each one is posted at that floor and no larger. What a swap
 * takes from them is the scenario's own business, and most take a fraction of one.
 */
export function standingOrders(solver) {
	const sources = ["EVM-97", "EVM-80002"]
	return [
		{ fillChain: "EVM-80002", tokenIn: "USDC", amountIn: "10", tokenOut: "cNGN", amountOut: dec(10 * solver.a), acceptedSources: sources },
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "10", tokenOut: "cNGN", amountOut: dec(10 * solver.b), acceptedSources: sources },
		{ fillChain: "EVM-97", tokenIn: "cNGN", amountIn: dec(10 * solver.c), tokenOut: "USDC", amountOut: "10", acceptedSources: sources },
	]
}

/** Extra solver 1 levels on BSC Chapel, above every standing order, each at the dust floor. */
export const EXTRA_LEVELS = {
	// Buy cNGN at 1590 and 1585.
	bids: [
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "10", tokenOut: "cNGN", amountOut: "15900", acceptedSources: ["EVM-97", "EVM-80002"] },
		{ fillChain: "EVM-97", tokenIn: "USDC", amountIn: "10", tokenOut: "cNGN", amountOut: "15850", acceptedSources: ["EVM-97", "EVM-80002"] },
	],
	// Sell cNGN at 1590.
	ask: [{ fillChain: "EVM-97", tokenIn: "cNGN", amountIn: "15900", tokenOut: "USDC", amountOut: "10", acceptedSources: ["EVM-97", "EVM-80002"] }],
}

/**
 * Each scenario runs against a freshly posted book: the standing orders, plus the extra
 * solver 1 levels it names. `minFills` is the fewest fill transactions it must take.
 *
 * `mixedPairs` scenarios put two pairs in one order, which #1311 forbids. They pass only against
 * a gateway without that rule, so they run only when named.
 */
export const SCENARIOS = {
	"same-chain": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		legs: [{ tokenIn: "USDC", amountIn: "0.5", tokenOut: "cNGN", minOut: "778.5" }],
		minFills: 1,
	},
	"cross-chain": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-80002",
		legs: [{ tokenIn: "USDC", amountIn: "0.4", tokenOut: "cNGN", minOut: "622" }],
		minFills: 1,
	},
	// 20000 cNGN is more than the 16000 solver 1's order takes, so it fills in parts.
	partial: {
		user: 1,
		source: "EVM-80002",
		dest: "EVM-97",
		legs: [{ tokenIn: "cNGN", amountIn: "20000", tokenOut: "USDC", minOut: "12.22" }],
		minFills: 2,
	},
	// Legs on two pairs, each reaching a different set of limit orders: leg 0 at 1565+ cNGN per
	// USDC (solver 1 and 2), leg 1 at up to ~1625 (all three).
	"multi-leg": {
		mixedPairs: true,
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		legs: [
			{ tokenIn: "USDC", amountIn: "0.5", tokenOut: "cNGN", minOut: "782.1" },
			{ tokenIn: "cNGN", amountIn: "2000", tokenOut: "USDC", minOut: "1.23" },
		],
		minFills: 2,
	},
	// Only solver 1 clears 1574 cNGN per USDC, and its best level takes 10 USDC of the 12:
	// several bids from one solver, best rate first. A ladder scenario cannot go much below the
	// floor one limit order is posted at.
	"same-solver-levels": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids"],
		legs: [{ tokenIn: "USDC", amountIn: "12", tokenOut: "cNGN", minOut: "18890.4" }],
		minFills: 2,
	},
	// Both pairs, solver 1's levels only on each side.
	"multi-leg-levels": {
		mixedPairs: true,
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids", "ask"],
		legs: [
			{ tokenIn: "USDC", amountIn: "12", tokenOut: "cNGN", minOut: "18890.4" },
			{ tokenIn: "cNGN", amountIn: "2000", tokenOut: "USDC", minOut: "1.2454" },
		],
		minFills: 2,
	},
	// One pair at two legs, 11 USDC against a level that takes 10: the second leg is sized from
	// what the first left.
	"multi-leg-same-input": {
		user: 0,
		source: "EVM-97",
		dest: "EVM-97",
		levels: ["bids"],
		legs: [
			{ tokenIn: "USDC", amountIn: "5.5", tokenOut: "cNGN", minOut: "8658.1" },
			{ tokenIn: "USDC", amountIn: "5.5", tokenOut: "cNGN", minOut: "8658.1" },
		],
		minFills: 2,
	},
}
