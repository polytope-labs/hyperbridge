import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { IntentGatewayContext } from "@/protocols/intents/types"
import type { HexString, Order } from "@/types"

// The gateway's release is read from a live node; here it is taken as supported.
vi.mock("@/protocols/intents/fillOrderCodec", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/protocols/intents/fillOrderCodec")>()),
	assertGatewayRelease: vi.fn(async () => {}),
}))

const { GasEstimator } = await import("@/protocols/intents/GasEstimator")

// A testnet chain, so gas is not priced through a swap quote.
const CHAIN = "EVM-97"
const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const USER = "0x9C7054b429f6b1dd35FD03e4fDC4f875Bc19931f" as HexString
const TOKEN = `0x000000000000000000000000${"46c85152bfe9f96829aa94755d9f915f9b10ef5f"}` as HexString

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

function estimatorWith(bundler: { batch: ReturnType<typeof vi.fn>; single: ReturnType<typeof vi.fn> }) {
	const chain = {
		config: { stateMachineId: CHAIN },
		client: {
			getGasPrice: vi.fn().mockResolvedValue(1_000_000n),
			getBlock: vi.fn().mockResolvedValue({ baseFeePerGas: 900_000n }),
		},
		configService: {
			getIntentGatewayAddress: () => GATEWAY,
			getEntryPointV08Address: () => ENTRY_POINT,
		},
		getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: `0x${TOKEN.slice(26)}`, decimals: 6 }),
	}
	const ctx = {
		source: chain,
		dest: chain,
		bundlerUrl: "https://bundler.example",
		feeTokenCache: new Map(),
	} as unknown as IntentGatewayContext
	const crypto = {
		encodeERC7821Execute: () => "0x" as HexString,
		sendBundlerBatch: bundler.batch,
		sendBundler: bundler.single,
	} as unknown as CryptoUtils

	const estimator = new GasEstimator(ctx, crypto)
	// Storage-slot discovery traces calls on a live node.
	vi.spyOn(estimator, "buildStateOverride").mockResolvedValue({ viem: [], bundler: {} })
	return estimator
}

const rejecting = () => ({
	batch: vi.fn().mockRejectedValue(new Error("batch unsupported")),
	single: vi.fn().mockRejectedValue(new Error("AA23 reverted")),
})

describe("GasEstimator.estimateFillOrder when the bundler fails to estimate", () => {
	beforeEach(() => vi.clearAllMocks())

	it("throws the bundler's error when a bundler estimate is required", async () => {
		const estimator = estimatorWith(rejecting())

		await expect(estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })).rejects.toThrow(
			"Bundler gas estimation failed: AA23 reverted",
		)
	})

	it("returns fixed gas limits when it is not", async () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		const estimator = estimatorWith(rejecting())

		const estimate = await estimator.estimateFillOrder({ order: ORDER })

		expect(estimate.callGasLimit).toBe(500_000n)
		expect(estimate.verificationGasLimit).toBe(100_000n)
		expect(estimate.preVerificationGas).toBe(100_000n)
		warn.mockRestore()
	})

	it("returns the bundler's figures when the estimate succeeds", async () => {
		const estimator = estimatorWith({
			batch: vi
				.fn()
				.mockResolvedValue([
					{ callGasLimit: "0x186a0", verificationGasLimit: "0x186a0", preVerificationGas: "0x186a0" },
				]),
			single: vi.fn(),
		})

		const estimate = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		expect(estimate.callGasLimit).toBe(160_000n)
		expect(estimate.verificationGasLimit).toBe(105_000n)
		expect(estimate.preVerificationGas).toBe(105_000n)
	})
})
