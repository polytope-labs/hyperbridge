import { concat, encodePacked } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { CryptoUtils, type HexString, type PackedUserOperation } from "@hyperbridge/sdk"
import {
	ENTRY_POINT_V08_ADDRESS,
	encodePhantomBidPaymasterAndData,
	recoverBidSignerViem,
} from "@hyperbridge/sdk/intents-helpers"
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

	// A bid built on the real-bid path carries a 234-byte-plus paymasterAndData (the Simplex
	// paymaster's Permit2 payload, then the declaration). The field is part of the EIP-712 payload
	// both sides hash, so the ethers encoder has to take the long bytes value to the same digest
	// viem does — otherwise every sponsored bid would recover to a stranger and be dropped.
	it("recovers the solver from a Permit2-sponsored bid the same way viem does", async () => {
		const solver = privateKeyToAccount(SOLVER_KEY)
		const paymasterData = encodePacked(
			["uint8", "address", "uint256", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
			[
				2,
				"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
				5_000_000n,
				2n ** 200n + 1n,
				1_800_000_000n,
				27,
				`0x${"aa".repeat(32)}`,
				`0x${"bb".repeat(32)}`,
			],
		)
		const sponsorship = encodePacked(
			["address", "uint128", "uint128", "bytes"],
			["0x0f9c4b1a2d3e4f5061728394a5b6c7d8e9f01234", 200_000n, 40_000n, paymasterData],
		)
		const userOp = {
			...userOpFor(solver.address as HexString),
			paymasterAndData: encodePhantomBidPaymasterAndData({ sponsorship, acceptedSourceChains: ["EVM-1"] }),
		}
		const solverSignature = (await solver.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
		)) as HexString

		const viaEthers = await recoverBidSignerVm2(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)
		const viaViem = await recoverBidSignerViem(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)

		expect(viaEthers!.toLowerCase()).toBe(solver.address.toLowerCase())
		expect(viaViem!.toLowerCase()).toBe(solver.address.toLowerCase())
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
