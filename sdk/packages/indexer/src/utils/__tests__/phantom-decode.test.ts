import { concat } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { CryptoUtils, type HexString, type PackedUserOperation } from "@hyperbridge/sdk"
import { ENTRY_POINT_V08_ADDRESS, recoverBidSignerViem } from "@hyperbridge/sdk/intents-helpers"
import { recoverBidSignerVm2 } from "@/utils/phantom-decode"

// A solver signs its bid with viem (simplex), and the indexer recovers it with ethers, so the two
// implementations must agree on the userOpHash digest down to the byte. If they ever drift, recovery
// yields some unrelated address and every phantom bid is silently rejected — hence signing here the
// way simplex does and recovering the way the indexer does.
const CHAIN_ID = 8453n
const SOLVER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const COMMITMENT = `0x${"11".repeat(32)}` as HexString

function userOpFor(sender: HexString): PackedUserOperation {
	return {
		sender,
		nonce: 42n,
		initCode: "0x",
		callData: "0xdeadbeef",
		accountGasLimits: `0x${"11".repeat(32)}`,
		preVerificationGas: 50_000n,
		gasFees: `0x${"22".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x",
	}
}

describe("recoverBidSignerVm2", () => {
	it("recovers the solver from a bid signed the way simplex signs it", async () => {
		const solver = privateKeyToAccount(SOLVER_KEY)
		const userOp = userOpFor(solver.address as HexString)
		const solverSignature = (await solver.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
		)) as HexString

		const recovered = await recoverBidSignerVm2(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)

		expect(recovered!.toLowerCase()).toBe(solver.address.toLowerCase())
	})

	it("agrees with the SDK's viem recovery on the same bid", async () => {
		const solver = privateKeyToAccount(SOLVER_KEY)
		const userOp = userOpFor(solver.address as HexString)
		const solverSignature = (await solver.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
		)) as HexString
		// The bid carries `commitment ‖ solverSignature`; both recoveries take the signature alone.
		const signed = { ...userOp, signature: concat([COMMITMENT, solverSignature]) as HexString }

		const viaEthers = await recoverBidSignerVm2(signed, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)
		const viaViem = await recoverBidSignerViem(signed, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)

		expect(viaEthers!.toLowerCase()).toBe(viaViem!.toLowerCase())
	})

	it("binds the signature to the operation it was signed for", async () => {
		const solver = privateKeyToAccount(SOLVER_KEY)
		const userOp = userOpFor(solver.address as HexString)
		const solverSignature = (await solver.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
		)) as HexString

		// Swapping in different calldata changes the digest, so the solver no longer recovers.
		const tampered = await recoverBidSignerVm2(
			{ ...userOp, callData: "0xc0ffee" },
			ENTRY_POINT_V08_ADDRESS,
			CHAIN_ID,
			solverSignature,
		)
		// As does replaying it onto another chain.
		const otherChain = await recoverBidSignerVm2(userOp, ENTRY_POINT_V08_ADDRESS, 1n, solverSignature)

		expect(tampered!.toLowerCase()).not.toBe(solver.address.toLowerCase())
		expect(otherChain!.toLowerCase()).not.toBe(solver.address.toLowerCase())
	})

	it("returns null for a malformed signature", async () => {
		const solver = privateKeyToAccount(SOLVER_KEY)
		const userOp = userOpFor(solver.address as HexString)

		expect(await recoverBidSignerVm2(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, "0xdeadbeef")).toBeNull()
	})
})
