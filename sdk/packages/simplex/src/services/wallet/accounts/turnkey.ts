import type { HexString } from "@hyperbridge/sdk"
import { Turnkey } from "@turnkey/sdk-server"
import { createAccount, signAuthorization } from "@turnkey/viem"
import type { TurnkeySignerConfig, Signer } from "../types"
import { viemSigner } from "./viem"

/**
 * Signs through Turnkey. The viem adapter covers everything except EIP-7702:
 * Turnkey encodes the authorization tuple natively, so the policy engine sees
 * (chainId, delegate, nonce) instead of an opaque digest.
 */
export async function turnkeySigner(config: TurnkeySignerConfig): Promise<Signer> {
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
		...viemSigner(account),
		mode: "turnkey",
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
	}
}
