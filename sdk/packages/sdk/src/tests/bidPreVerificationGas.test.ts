import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { BID_PVG_ESTIMATION_ACCOUNT_CODE, GasEstimator } from "@/protocols/intents/GasEstimator"
import { BundlerMethod, type IntentGatewayContext } from "@/protocols/intents/types"
import type { BidPreVerificationGasParams, HexString } from "@/types"
import { describe, expect, it, vi } from "vitest"

const SOLVER = "0x21426d68a9e5df153fe75ce0fed20173ebcb80ef" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString

const BID: BidPreVerificationGasParams = {
	solverAccount: SOLVER,
	nonce: 7n << 64n,
	entryPointAddress: ENTRY_POINT,
	callGasLimit: 1_200_000n,
	verificationGasLimit: 105_000n,
	maxFeePerGas: 8_750_000n,
	maxPriorityFeePerGas: 1_250_000n,
	callData: "0xe9ae5c53" as HexString,
	// Paymaster, its verification and postOp gas limits, then its data.
	paymasterAndData:
		"0x15b3b03c870c7ef252029c35a12d3b339f5c8d7f00000000000000000000000000030d4000000000000000000000000000009c4002" as HexString,
}

function estimatorAt(baseFeePerGas: bigint | null, preVerificationGas = "0x173a2") {
	const sendBundler = vi.fn().mockResolvedValue({
		preVerificationGas,
		verificationGasLimit: "0x19a28",
		callGasLimit: "0x124f80",
	})
	const ctx = {
		dest: { client: { getBlock: vi.fn().mockResolvedValue({ baseFeePerGas }) } },
	} as unknown as IntentGatewayContext
	const estimator = new GasEstimator(ctx, { sendBundler } as unknown as CryptoUtils)
	return { estimator, sendBundler }
}

describe("GasEstimator.estimateBidPreVerificationGas", () => {
	it("prices the signed op at the bundle's gas price, not its max fee", async () => {
		const { estimator, sendBundler } = estimatorAt(5_000_000n)

		await estimator.estimateBidPreVerificationGas(BID)

		const [method, [userOp, entryPoint]] = sendBundler.mock.calls[0]
		expect(method).toBe(BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS)
		expect(entryPoint).toBe(ENTRY_POINT)
		// Base fee plus the tip: what the bundle pays per gas, below the 8.75M max fee.
		expect(BigInt(userOp.maxFeePerGas)).toBe(6_250_000n)
		expect(BigInt(userOp.maxPriorityFeePerGas)).toBe(1_250_000n)
	})

	it("caps the price at the op's max fee when the base fee has risen past it", async () => {
		const { estimator, sendBundler } = estimatorAt(8_000_000n)

		await estimator.estimateBidPreVerificationGas(BID)

		const [, [userOp]] = sendBundler.mock.calls[0]
		expect(BigInt(userOp.maxFeePerGas)).toBe(8_750_000n)
	})

	it("sends the bid's own bytes with a selected bid's full signature and no preVerificationGas", async () => {
		const { estimator, sendBundler } = estimatorAt(5_000_000n)

		await estimator.estimateBidPreVerificationGas(BID)

		const [, [userOp, , stateOverride]] = sendBundler.mock.calls[0]
		const expected = CryptoUtils.prepareBundlerCall({
			sender: SOLVER,
			nonce: BID.nonce,
			initCode: "0x" as HexString,
			callData: BID.callData,
			accountGasLimits: CryptoUtils.packGasLimits(BID.verificationGasLimit, BID.callGasLimit),
			preVerificationGas: 0n,
			gasFees: CryptoUtils.packGasFees(1_250_000n, 6_250_000n),
			paymasterAndData: BID.paymasterAndData as HexString,
			signature: `0x${"ff".repeat(162)}` as HexString,
		})
		expect(userOp).toEqual(expected)
		expect(stateOverride).toEqual({ [SOLVER]: { code: BID_PVG_ESTIMATION_ACCOUNT_CODE } })
	})

	it("adds 10% headroom to the bundler's figure", async () => {
		const { estimator } = estimatorAt(5_000_000n, "0x173a2")

		expect(await estimator.estimateBidPreVerificationGas(BID)).toBe((0x173a2n * 110n) / 100n)
	})
})
