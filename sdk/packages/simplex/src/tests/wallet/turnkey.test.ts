import { describe, it, expect } from "vitest"
import { turnkeySigner } from "@/services/wallet/accounts/turnkey"
import {
	keccak256,
	toHex,
	verifyMessage,
	recoverAddress,
	createPublicClient,
	createWalletClient,
	http,
	concat,
	toRlp,
	zeroAddress,
} from "viem"
import { sepolia } from "viem/chains"
import type { HexString } from "@hyperbridge/sdk"

const hasTurnkeyEnv =
	process.env.TURNKEY_ORG_ID &&
	process.env.TURNKEY_API_PUBLIC_KEY &&
	process.env.TURNKEY_API_PRIVATE_KEY &&
	process.env.TURNKEY_SIGN_WITH

const config = {
	organizationId: process.env.TURNKEY_ORG_ID!,
	apiPublicKey: process.env.TURNKEY_API_PUBLIC_KEY!,
	apiPrivateKey: process.env.TURNKEY_API_PRIVATE_KEY!,
	signWith: process.env.TURNKEY_SIGN_WITH!,
}

const DELEGATION_INDICATOR_PREFIX = "0xef0100"

describe.skipIf(!hasTurnkeyEnv)("Turnkey signer", () => {
	it("should create a signing account with the correct address", async () => {
		const signer = await turnkeySigner(config)

		expect(signer.mode).toBe("turnkey")
		expect(signer.account.address.toLowerCase()).toBe(config.signWith.toLowerCase())
	})

	// Not part of the Signer interface — the solver never personal-signs — but the
	// account it is built on has to, since viem's wallet client leans on it.
	it("should sign and verify a message", async () => {
		const signer = await turnkeySigner(config)
		const messageHash = keccak256(toHex("hello turnkey"))

		const signature = await signer.account.signMessage!({ message: { raw: messageHash } })
		expect(signature).toMatch(/^0x/)
		expect(signature.length).toBe(132)

		const valid = await verifyMessage({
			address: signer.account.address,
			message: { raw: messageHash as `0x${string}` },
			signature: signature as `0x${string}`,
		})
		expect(valid).toBe(true)
	})

	it("should sign a raw hash and verify recovery", async () => {
		const signer = await turnkeySigner(config)
		const hash = keccak256(toHex("test hash"))

		const result = await signer.signRawHash(hash as HexString)
		expect(result.r).toMatch(/^0x/)
		expect(result.s).toMatch(/^0x/)
		expect([0, 1]).toContain(result.yParity)

		const v = BigInt(result.yParity) + 27n
		const recovered = await recoverAddress({
			hash: hash as `0x${string}`,
			signature: { r: result.r as `0x${string}`, s: result.s as `0x${string}`, v },
		})
		expect(recovered.toLowerCase()).toBe(config.signWith.toLowerCase())
	})

	it("should delegate via EIP-7702, verify, then revoke on Sepolia", async () => {
		const signer = await turnkeySigner(config)
		const rpcUrl = process.env.SEPOLIA!

		const publicClient = createPublicClient({ chain: sepolia, transport: http(rpcUrl) })
		const walletClient = createWalletClient({
			chain: sepolia,
			transport: http(rpcUrl),
			account: signer.account,
		})

		// Random contract address to delegate to
		const delegateTarget = "0x0000000000000000000000000000000000000001" as HexString
		const authorityAddress = signer.account.address

		// --- Delegate ---
		const nonce = await publicClient.getTransactionCount({ address: authorityAddress, blockTag: "pending" })
		const authNonce = nonce + 1
		const authHash = keccak256(
			concat(["0x05", toRlp([toHex(sepolia.id), delegateTarget, toHex(authNonce)])]),
		) as HexString
		const authSig = await signer.signRawHash(authHash)

		const authorization = {
			chainId: sepolia.id,
			address: delegateTarget,
			nonce: authNonce,
			r: authSig.r,
			s: authSig.s,
			yParity: authSig.yParity,
		}

		const delegateTxHash = await walletClient.sendTransaction({
			to: authorityAddress,
			value: 0n,
			authorizationList: [authorization],
			chain: sepolia,
			gas: 350_000n,
		})

		const delegateReceipt = await publicClient.waitForTransactionReceipt({ hash: delegateTxHash })
		expect(delegateReceipt.status).toBe("success")

		// Verify delegation
		const codeAfterDelegate = await publicClient.getCode({ address: authorityAddress })
		expect(codeAfterDelegate?.toLowerCase().startsWith(DELEGATION_INDICATOR_PREFIX)).toBe(true)
		const delegatedTo = "0x" + codeAfterDelegate!.slice(8)
		expect(delegatedTo.toLowerCase()).toBe(delegateTarget.toLowerCase())

		// --- Revoke (delegate to zero address) ---
		const revokeNonce = await publicClient.getTransactionCount({ address: authorityAddress, blockTag: "pending" })
		const revokeAuthNonce = revokeNonce + 1
		const revokeAuthHash = keccak256(
			concat(["0x05", toRlp([toHex(sepolia.id), zeroAddress, toHex(revokeAuthNonce)])]),
		) as HexString
		const revokeAuthSig = await signer.signRawHash(revokeAuthHash)

		const revokeAuthorization = {
			chainId: sepolia.id,
			address: zeroAddress as HexString,
			nonce: revokeAuthNonce,
			r: revokeAuthSig.r,
			s: revokeAuthSig.s,
			yParity: revokeAuthSig.yParity,
		}

		const revokeTxHash = await walletClient.sendTransaction({
			to: authorityAddress,
			value: 0n,
			authorizationList: [revokeAuthorization],
			chain: sepolia,
			gas: 350_000n,
		})

		const revokeReceipt = await publicClient.waitForTransactionReceipt({ hash: revokeTxHash })
		expect(revokeReceipt.status).toBe("success")

		// Verify delegation cleared
		const codeAfterRevoke = await publicClient.getCode({ address: authorityAddress })
		const isDelegationCleared = !codeAfterRevoke || codeAfterRevoke === "0x"
		expect(isDelegationCleared).toBe(true)
	}, 120_000)
})
