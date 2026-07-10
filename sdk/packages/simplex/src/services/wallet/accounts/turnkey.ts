import type { HexString } from "@hyperbridge/sdk"
import { Turnkey } from "@turnkey/sdk-server"
import { createAccount, signAuthorization } from "@turnkey/viem"
import { parseSignature } from "viem"
import type { TurnkeySignerConfig, SigningAccount } from "../types"

export async function createTurnkeySigningAccount(config: TurnkeySignerConfig): Promise<SigningAccount> {
	const turnkey = new Turnkey({
		defaultOrganizationId: config.organizationId,
		apiBaseUrl: "https://api.turnkey.com",
		apiPrivateKey: config.apiPrivateKey,
		apiPublicKey: config.apiPublicKey,
	})

	const account = await createAccount({
		client: turnkey.apiClient(),
		organizationId: config.organizationId,
		signWith: config.signWith,
	})

	return {
		mode: "turnkey",
		account,
		signMessage: (messageHash: HexString, _chainId: number) =>
			account.signMessage({ message: { raw: messageHash } }),
		signTypedData: (typedData: unknown, _chainId?: number) =>
			account.signTypedData(typedData as Parameters<typeof account.signTypedData>[0]) as Promise<HexString>,
		signRawHash: async (hash: HexString) => {
			const raw = await account.sign!({ hash })
			const sig = parseSignature(raw)
			const yParity =
				sig.yParity ?? (sig.v !== undefined ? Number(sig.v >= 27n ? sig.v - 27n : sig.v) : undefined)
			if (yParity !== 0 && yParity !== 1) {
				throw new Error("Failed to derive yParity from Turnkey signature")
			}
			return {
				r: sig.r as HexString,
				s: sig.s as HexString,
				yParity,
			}
		},
		// Sign the authorization tuple through Turnkey's structured 7702 encoding so the
		// policy engine sees (chainId, delegate, nonce) instead of an opaque digest.
		signAuthorization: async (auth) => {
			const signed = await signAuthorization(
				turnkey.apiClient(),
				{ contractAddress: auth.contractAddress, chainId: auth.chainId, nonce: auth.nonce },
				config.organizationId,
				config.signWith,
			)
			const yParity = signed.yParity ?? (signed.v !== undefined ? Number(signed.v - 27n) : undefined)
			if (yParity !== 0 && yParity !== 1) {
				throw new Error("Failed to derive yParity from Turnkey authorization signature")
			}
			return { r: signed.r as HexString, s: signed.s as HexString, yParity }
		},
		sendEip7702DelegationTransaction: async (args) =>
			(await args.walletClient.sendTransaction({
				to: args.authorityAddress,
				value: 0n,
				authorizationList: [args.authorization],
				chain: args.walletClient.chain,
				gas: args.gasFloor,
			})) as HexString,
	}
}
