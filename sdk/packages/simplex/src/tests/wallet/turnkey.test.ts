import { describe, it, expect } from "vitest"
import { turnkeySigner } from "@/services/wallet/accounts/turnkey"
import { accountFor } from "@/services/wallet/account"
import { recoverAddress, createPublicClient, createWalletClient, http, zeroAddress } from "viem"
import { hashAuthorization } from "viem/utils"
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
/** Arbitrary delegate target for the signing checks. */
const DELEGATE_TARGET = "0x0000000000000000000000000000000000000001" as HexString

describe.skipIf(!hasTurnkeyEnv)("Turnkey signer", () => {
	it("should create a signing account with the correct address", async () => {
		const signer = await turnkeySigner(config)

		expect(signer.mode).toBe("turnkey")
		expect(signer.address.toLowerCase()).toBe(config.signWith.toLowerCase())
	})

	// Turnkey encodes the authorization tuple natively, so this exercises its
	// structured path rather than a digest.
	it("should sign an EIP-7702 authorization and verify recovery", async () => {
		const signer = await turnkeySigner(config)
		const auth = { chainId: sepolia.id, contractAddress: DELEGATE_TARGET, nonce: 7 }

		const result = await signer.signAuthorization(auth)
		expect(result.r).toMatch(/^0x/)
		expect(result.s).toMatch(/^0x/)
		expect([0, 1]).toContain(result.yParity)

		const recovered = await recoverAddress({
			hash: hashAuthorization({ address: DELEGATE_TARGET, chainId: sepolia.id, nonce: 7 }),
			signature: { r: result.r as `0x${string}`, s: result.s as `0x${string}`, v: BigInt(result.yParity) + 27n },
		})
		expect(recovered.toLowerCase()).toBe(config.signWith.toLowerCase())
	})

	it("should delegate via EIP-7702, verify, then revoke on Sepolia", async () => {
		const signer = await turnkeySigner(config)
		const rpcUrl = process.env.SEPOLIA!

		const publicClient = createPublicClient({ chain: sepolia, transport: http(rpcUrl) })
		// The same derivation ChainClientManager does: Signer in, viem account out.
		const walletClient = createWalletClient({
			chain: sepolia,
			transport: http(rpcUrl),
			account: accountFor(signer),
		})

		const delegateTarget = DELEGATE_TARGET
		const authorityAddress = signer.address as `0x${string}`

		// --- Delegate ---
		const nonce = await publicClient.getTransactionCount({ address: authorityAddress, blockTag: "pending" })
		const authNonce = nonce + 1
		const authSig = await signer.signAuthorization({
			chainId: sepolia.id,
			contractAddress: delegateTarget,
			nonce: authNonce,
		})

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
		const revokeAuthSig = await signer.signAuthorization({
			chainId: sepolia.id,
			contractAddress: zeroAddress,
			nonce: revokeAuthNonce,
		})

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
