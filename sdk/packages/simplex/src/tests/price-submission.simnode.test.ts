/**
 * Integration test for the fee-based price submission system.
 *
 * Tests IntentsCoprocessor and FXFiller.submitInitialPrices() against a running
 * hyperbridge simnode.
 *
 * Requires a running hyperbridge simnode:
 *   target/release/hyperbridge simnode --chain gargantua-2000 --rpc-port 9990 --tmp
 *
 * Run with:
 *   SIMNODE_URL=ws://127.0.0.1:9990 pnpm test:prices
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest"
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { IntentsCoprocessor } from "@hyperbridge/sdk"
import type { HexString } from "@hyperbridge/sdk"
import { parseUnits, keccak256, encodePacked } from "viem"
import { FXFiller } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"

// Alice's dev URI for polkadot.js Keyring
const ALICE_URI = "//Alice"
// Alice's raw sr25519 seed (hex) for the SDK's IntentsCoprocessor
const ALICE_SEED = "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"

// Test token symbols
const STABLE_SYMBOL = "USDC"
const EXOTIC_SYMBOL = "cNGN"

/**
 * Compute the on-chain pair ID from two token symbols: keccak256("base/quote")
 * This matches both the pallet's TokenPair::pair_id() and FXFiller.computeSymbolPairId().
 */
function computeSymbolPairId(baseSymbol: string, quoteSymbol: string): HexString {
	return keccak256(encodePacked(["string"], [`${baseSymbol}/${quoteSymbol}`])) as HexString
}

/** Send a raw RPC to the simnode */
async function rpc(api: ApiPromise, method: string, params: any[] = []): Promise<any> {
	return (api as any)._rpcCore.provider.send(method, params)
}

/** Create and finalize a block on the simnode */
async function createBlock(api: ApiPromise): Promise<void> {
	const block = await rpc(api, "engine_createBlock", [true, false])
	await rpc(api, "engine_finalizeBlock", [block.hash])
}

/**
 * Submit an extrinsic, seal it into a block, and check for success.
 */
async function submitAndSeal(
	api: ApiPromise,
	extrinsic: any,
	signer: any,
): Promise<{ success: boolean; error?: string }> {
	await extrinsic.signAsync(signer)
	await api.rpc.author.submitExtrinsic(extrinsic)
	await createBlock(api)

	// Check events for dispatch errors
	const header = await api.rpc.chain.getHeader()
	const apiAt = await api.at(header.hash)
	const events: any[] = (await apiAt.query.system.events()) as any
	for (const { event } of events) {
		if (event.section === "system" && event.method === "ExtrinsicFailed") {
			return { success: false, error: `Dispatch error: ${event.data[0].toString()}` }
		}
	}
	return { success: true }
}

/** Wrap a call in sudo, submit from Alice, and seal a block. */
async function sudoAndSeal(api: ApiPromise, call: any): Promise<void> {
	const keyring = new Keyring({ type: "sr25519" })
	const alice = keyring.addFromUri(ALICE_URI)
	const sudoCall = api.tx.sudo.sudo(call)
	const result = await submitAndSeal(api, sudoCall, alice)
	if (!result.success) throw new Error(result.error || "sudo call failed")
}

describe("Price Submission (simnode)", () => {
	let api: ApiPromise
	let coprocessor: IntentsCoprocessor
	// Ask pair: stable/exotic (USDC/cNGN)
	let askPairId: HexString
	// Bid pair: exotic/stable (cNGN/USDC)
	let bidPairId: HexString

	beforeAll(async () => {
		api = await ApiPromise.create({
			provider: new WsProvider(SIMNODE_URL),
			typesBundle: {
				spec: {
					gargantua: { hasher: keccakAsU8a },
				},
			},
		})

		// Compute symbol-based pair IDs
		askPairId = computeSymbolPairId(STABLE_SYMBOL, EXOTIC_SYMBOL)
		bidPairId = computeSymbolPairId(EXOTIC_SYMBOL, STABLE_SYMBOL)
		console.log("Ask Pair ID (USDC/cNGN):", askPairId)
		console.log("Bid Pair ID (cNGN/USDC):", bidPairId)

		// Create IntentsCoprocessor using Alice's dev seed
		coprocessor = IntentsCoprocessor.fromApi(api, ALICE_SEED)

		// ── Governance setup via sudo ───────────────────────────────
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPriceSubmissionFee(100_000_000_000_000))
		console.log("Price submission fee set")
	}, 60_000)

	afterAll(async () => {
		await coprocessor.disconnect()
		await api.disconnect()
	})

	it("should submit ask prices via IntentsCoprocessor.submitPairPrice()", async () => {
		const entries = [
			{
				amount: parseUnits("0", 18),
				price: parseUnits("1414.5", 18),
			},
			{
				amount: parseUnits("1000", 18),
				price: parseUnits("1420", 18),
			},
		]

		const submitPromise = coprocessor.submitPairPrice(askPairId, entries)
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const result = await submitPromise
		console.log("submitPairPrice (ask) result:", result)
		expect(result.success).toBe(true)

		// Verify via RPC
		const prices: any[] = await rpc(api, "intents_getPairPrices", [askPairId])
		console.log("RPC ask prices:", prices)

		expect(prices.length).toBe(2)
		expect(prices[0].amount).toBe("0")
		expect(prices[0].price).toBe("1414.5")
		expect(prices[1].amount).toBe("1000")
		expect(prices[1].price).toBe("1420")
	}, 30_000)

	it("should submit bid prices via IntentsCoprocessor.submitPairPrice()", async () => {
		// Bid pair (cNGN/USDC): amount threshold in exotic tokens, price = USDC per 1 cNGN
		// Use amount=500 to avoid overlap with FXFiller's bid point 0 (amount=0, price=0.000625)
		const entries = [
			{
				amount: parseUnits("500", 18),
				price: parseUnits("0.000625", 18), // 1/1600
			},
		]

		const submitPromise = coprocessor.submitPairPrice(bidPairId, entries)
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const result = await submitPromise
		console.log("submitPairPrice (bid) result:", result)
		expect(result.success).toBe(true)

		const prices: any[] = await rpc(api, "intents_getPairPrices", [bidPairId])
		console.log("RPC bid prices:", prices)

		expect(prices.length).toBe(1)
		expect(prices[0].amount).toBe("500")
		expect(prices[0].price).toBe("0.000625")
	}, 30_000)

	it("should submit both ask and bid prices via FXFiller.submitInitialPrices()", async () => {
		// Define curves as they'd appear in the TOML config
		const askCurvePoints = [
			{ amount: "0", price: "1500" },
			{ amount: "1000", price: "1510" },
			{ amount: "5000", price: "1525" },
		]
		const bidCurvePoints = [
			{ amount: "0", price: "1600" },
			{ amount: "1000", price: "1610" },
			{ amount: "5000", price: "1625" },
		]

		const askPricePolicy = new FillerPricePolicy({ points: askCurvePoints })
		const bidPricePolicy = new FillerPricePolicy({ points: bidCurvePoints })

		// Mock services — contractService.getTokenSymbol() is used to read symbols
		const DUMMY_CHAIN = "EVM-1"
		const DUMMY_EXOTIC_ADDR = "0x000000000000000000000000000000000000bbbb" as HexString
		const DUMMY_USDC_ADDR = "0x000000000000000000000000000000000000aaaa" as HexString

		const mockConfigService = {
			getUsdcAsset: () => DUMMY_USDC_ADDR,
		} as any

		const mockContractService = {
			getTokenSymbol: async (address: string, _chain: string) => {
				if (address.toLowerCase() === DUMMY_EXOTIC_ADDR.toLowerCase()) return EXOTIC_SYMBOL
				if (address.toLowerCase() === DUMMY_USDC_ADDR.toLowerCase()) return STABLE_SYMBOL
				throw new Error(`Unknown token address: ${address}`)
			},
		} as any

		const fxFiller = new FXFiller(
			{ signMessage: async () => "0x" as HexString, signRawHash: async () => ({ r: "0x" as HexString, s: "0x" as HexString, yParity: 0 }) } as any,
			mockConfigService,
			{} as any, // clientManager — not used by submitInitialPrices
			mockContractService,
			bidPricePolicy,
			askPricePolicy,
			"5000",
			{ [DUMMY_CHAIN]: DUMMY_EXOTIC_ADDR },
			{ [DUMMY_CHAIN]: DUMMY_USDC_ADDR },
		)

		// submitInitialPrices will:
		// 1. Read symbols from chain (via mocked clients)
		// 2. Compute ask pair ID: keccak256(keccak256("USDC") || keccak256("cNGN"))
		// 3. Compute bid pair ID: keccak256(keccak256("cNGN") || keccak256("USDC"))
		// 4. Submit ask entries to ask pair
		// 5. Submit bid entries to bid pair
		const submitPromise = fxFiller.submitInitialPrices(Promise.resolve(coprocessor))
		// Need to seal blocks for each extrinsic:
		// submitAsk + submitBid = 2 blocks
		for (let i = 0; i < 2; i++) {
			await new Promise((r) => setTimeout(r, 300))
			await createBlock(api)
		}
		await submitPromise

		// Verify ask prices (overwrite semantics — only this filler's latest entries)
		const askPrices: any[] = await rpc(api, "intents_getPairPrices", [askPairId])
		console.log("FXFiller ask prices:", askPrices)

		// askPricePolicy has 3 points → filler's entries overwritten to 3
		expect(askPrices.length).toBeGreaterThanOrEqual(3)

		// Find the FXFiller entries by their known prices
		const ask1500 = askPrices.find((e: any) => e.price === "1500")
		const ask1510 = askPrices.find((e: any) => e.price === "1510")
		const ask1525 = askPrices.find((e: any) => e.price === "1525")
		expect(ask1500).toBeDefined()
		expect(ask1500.amount).toBe("0")
		expect(ask1510).toBeDefined()
		expect(ask1510.amount).toBe("1000")
		expect(ask1525).toBeDefined()
		expect(ask1525.amount).toBe("5000")

		// Verify bid prices (overwrite semantics — only this filler's latest entries)
		const bidPrices: any[] = await rpc(api, "intents_getPairPrices", [bidPairId])
		console.log("FXFiller bid prices:", bidPrices)

		// bidPricePolicy has 3 points → filler's entries overwritten to 3
		expect(bidPrices.length).toBeGreaterThanOrEqual(3)

		// Bid point 0: amount=0 USD, price=1600 exotic/USD
		//   → exoticAmount = 0*1600 = 0, stablePerExotic = 1/1600 = 0.000625
		const bid0 = bidPrices.find((e: any) => e.price === "0.000625")
		expect(bid0).toBeDefined()
		expect(bid0.amount).toBe("0")

		// Bid point 1: amount=1000 USD, price=1610 exotic/USD
		//   → exoticAmount = 1000*1610 = 1610000
		const bid1 = bidPrices.find((e: any) => e.amount === "1610000")
		expect(bid1).toBeDefined()

		// Bid point 2: amount=5000 USD, price=1625 exotic/USD
		//   → exoticAmount = 5000*1625 = 8125000
		const bid2 = bidPrices.find((e: any) => e.amount === "8125000")
		expect(bid2).toBeDefined()
	}, 60_000)

	it("should overwrite prices on re-submission", async () => {
		// Submit new prices for the ask pair — should overwrite the previous ones
		const newEntries = [
			{
				amount: parseUnits("0", 18),
				price: parseUnits("1500", 18),
			},
			{
				amount: parseUnits("2000", 18),
				price: parseUnits("1520", 18),
			},
		]

		const submitPromise = coprocessor.submitPairPrice(askPairId, newEntries)
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const result = await submitPromise
		console.log("Re-submission result:", result)
		expect(result.success).toBe(true)

		// Verify the old prices are gone and only the new ones remain for this filler
		const prices: any[] = await rpc(api, "intents_getPairPrices", [askPairId])
		console.log("Prices after re-submission:", prices)

		// Find our new entries
		const entry1500 = prices.find((e: any) => e.price === "1500")
		const entry1520 = prices.find((e: any) => e.price === "1520")
		expect(entry1500).toBeDefined()
		expect(entry1500.amount).toBe("0")
		expect(entry1520).toBeDefined()
		expect(entry1520.amount).toBe("2000")

		// The old prices (1414.5, 1420) should no longer exist for this filler
		const old1414 = prices.find((e: any) => e.price === "1414.5")
		expect(old1414).toBeUndefined()
	}, 30_000)
})
