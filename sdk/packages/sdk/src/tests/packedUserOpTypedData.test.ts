import { describe, it, expect } from "vitest"
import { hashTypedData } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { HexString, PackedUserOperation } from "@/types"

const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const CHAIN_ID = 8453n

function makeUserOp(): PackedUserOperation {
	return {
		sender: "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
		nonce: (0xabcdefn << 64n) | 7n,
		initCode: "0x" as HexString,
		callData: "0xe9ae5c53" as HexString,
		accountGasLimits: "0x000000000000000000000000000186a0000000000000000000000000000f4240" as HexString,
		preVerificationGas: 50_000n,
		gasFees: "0x0000000000000000000000003b9aca000000000000000000000000003b9aca00" as HexString,
		paymasterAndData: "0x" as HexString,
		signature: "0x" as HexString,
	}
}

describe("packedUserOpTypedData", () => {
	it("typed-data digest equals the EntryPoint v0.8 userOpHash", () => {
		const userOp = makeUserOp()
		const typed = CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT, CHAIN_ID)
		expect(hashTypedData(typed)).toBe(CryptoUtils.computeUserOpHash(userOp, ENTRY_POINT, CHAIN_ID))
	})

	it("signing the typed data equals signing the raw userOpHash", async () => {
		const account = privateKeyToAccount("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d")
		const userOp = makeUserOp()

		const typedSignature = await account.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT, CHAIN_ID),
		)
		const rawSignature = await account.sign({
			hash: CryptoUtils.computeUserOpHash(userOp, ENTRY_POINT, CHAIN_ID),
		})
		expect(typedSignature).toBe(rawSignature)
	})
})
