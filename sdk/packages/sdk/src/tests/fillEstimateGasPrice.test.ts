import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { SELECT_SOLVER_TYPEHASH, type CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { IntentGatewayContext } from "@/protocols/intents/types"
import type { HexString, Order } from "@/types"

// The gateway's release is read from a live node; here it is taken as supported.
vi.mock("@/protocols/intents/fillOrderCodec", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/protocols/intents/fillOrderCodec")>()),
	assertGatewayRelease: vi.fn(async () => {}),
}))

vi.mock("@/protocols/intents/rundlerFees", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@/protocols/intents/rundlerFees")>()
	return { ...actual, fetchRundlerPriorityFee: vi.fn(actual.fetchRundlerPriorityFee) }
})

const { GasEstimator } = await import("@/protocols/intents/GasEstimator")
const { fetchRundlerPriorityFee } = await import("@/protocols/intents/rundlerFees")

// A testnet chain, so gas is not priced through a swap quote.
const CHAIN = "EVM-97"
const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const USER = "0x9C7054b429f6b1dd35FD03e4fDC4f875Bc19931f" as HexString
const TOKEN = `0x000000000000000000000000${"46c85152bfe9f96829aa94755d9f915f9b10ef5f"}` as HexString

const GAS_PRICE = 1_000_000n
const BASE_FEE = 900_000n
// The chain estimate with the default 8% priority and 10% max fee bumps.
const CHAIN_FEES = { maxPriorityFeePerGas: 1_080_000n, maxFeePerGas: 1_100_000n }

const ORDER: Order = {
	user: USER,
	source: CHAIN,
	destination: CHAIN,
	deadline: 65337297000n,
	nonce: 0n,
	fees: 0n,
	session: USER,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: TOKEN, amount: 1_000n }],
	output: { beneficiary: USER, assets: [{ token: TOKEN, amount: 990n }], call: "0x" },
}

const GAS_ESTIMATE = { callGasLimit: "0x1", verificationGasLimit: "0x1", preVerificationGas: "0x1" }

function estimatorWith(bundlerUrl: string, batch: ReturnType<typeof vi.fn>) {
	const chain = {
		config: { stateMachineId: CHAIN },
		client: {
			getGasPrice: vi.fn().mockResolvedValue(GAS_PRICE),
			getBlock: vi.fn().mockResolvedValue({ baseFeePerGas: BASE_FEE }),
			readContract: vi.fn().mockResolvedValue(SELECT_SOLVER_TYPEHASH),
		},
		configService: {
			getIntentGatewayAddress: () => GATEWAY,
			getEntryPointAddress: () => ENTRY_POINT,
		},
		getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: `0x${TOKEN.slice(26)}`, decimals: 6 }),
	}
	const ctx = { source: chain, dest: chain, bundlerUrl, feeTokenCache: new Map() } as unknown as IntentGatewayContext
	const crypto = {
		encodeERC7821Execute: () => "0x" as HexString,
		sendBundlerBatch: batch,
		sendBundler: vi.fn(),
	} as unknown as CryptoUtils

	const estimator = new GasEstimator(ctx, crypto)
	// Storage-slot discovery traces calls on a live node.
	vi.spyOn(estimator, "buildStateOverride").mockResolvedValue({ viem: [], bundler: {} })
	return estimator
}

/** Stubs the bundler's `rundler_maxPriorityFeePerGas`, which is fetched outside the estimate batch. */
function stubRundler(reply: Record<string, unknown>) {
	const fetch = vi.fn(async () => ({ json: async () => ({ jsonrpc: "2.0", id: 1, ...reply }) }))
	vi.stubGlobal("fetch", fetch)
	return fetch
}

beforeEach(() => vi.mocked(fetchRundlerPriorityFee).mockClear())
afterEach(() => vi.unstubAllGlobals())

describe("GasEstimator.estimateFillOrder gas price", () => {
	it("raises the fees to a rundler bundler's priority fee when it is above the chain estimate", async () => {
		const fetch = stubRundler({ result: "0x5f5e100" })
		const estimator = estimatorWith("https://rundler.example", vi.fn().mockResolvedValue([GAS_ESTIMATE]))

		const estimate = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		expect(estimate.maxPriorityFeePerGas).toBe(108_000_000n)
		expect(estimate.maxFeePerGas).toBe(BASE_FEE + BASE_FEE / 10n + 108_000_000n)
		expect(fetch).toHaveBeenCalledWith("https://rundler.example", expect.anything())
		expect(fetchRundlerPriorityFee).toHaveBeenCalledWith("https://rundler.example")
	})

	it("keeps the chain estimate and stops asking when the bundler does not serve rundler fees", async () => {
		const fetch = stubRundler({ error: { code: -32601, message: "Method not found" } })
		const estimator = estimatorWith("https://other-bundler.example", vi.fn().mockResolvedValue([GAS_ESTIMATE]))

		const first = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })
		const second = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		expect(first).toMatchObject(CHAIN_FEES)
		expect(second).toMatchObject(CHAIN_FEES)
		expect(fetch).toHaveBeenCalledOnce()
	})

	it("prices a Pimlico bundler from Pimlico's gas price without asking for rundler fees", async () => {
		const fetch = stubRundler({ result: "0x5f5e100" })
		const batch = vi
			.fn()
			.mockResolvedValue([GAS_ESTIMATE, { fast: { maxFeePerGas: "0x2dc6c0", maxPriorityFeePerGas: "0x1e8480" } }])
		const estimator = estimatorWith("https://api.pimlico.io/v2/97/rpc?apikey=k", batch)

		const estimate = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		expect(estimate.maxFeePerGas).toBe(3_300_000n)
		expect(estimate.maxPriorityFeePerGas).toBe(2_160_000n)
		expect(batch.mock.calls[0][0].map((request: { method: string }) => request.method)).toEqual([
			"eth_estimateUserOperationGas",
			"pimlico_getUserOperationGasPrice",
		])
		expect(fetch).not.toHaveBeenCalled()
		expect(fetchRundlerPriorityFee).not.toHaveBeenCalled()
	})

	it("prices an Alchemy bundler with its own buffers without the rundler floor", async () => {
		const fetch = stubRundler({ result: "0x5f5e100" })
		const batch = vi.fn().mockResolvedValue([GAS_ESTIMATE, "0x5f5e100"])
		const estimator = estimatorWith("https://bnb-testnet.g.alchemy.com/v2/k", batch)

		const estimate = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		// A 25% priority bump off Arbitrum, and a 50% base fee buffer.
		expect(estimate.maxPriorityFeePerGas).toBe(125_000_000n)
		expect(estimate.maxFeePerGas).toBe(BASE_FEE + BASE_FEE / 2n + 125_000_000n)
		expect(batch.mock.calls[0][0].map((request: { method: string }) => request.method)).toEqual([
			"eth_estimateUserOperationGas",
			"rundler_maxPriorityFeePerGas",
		])
		expect(fetch).not.toHaveBeenCalled()
		expect(fetchRundlerPriorityFee).not.toHaveBeenCalled()
	})
})
