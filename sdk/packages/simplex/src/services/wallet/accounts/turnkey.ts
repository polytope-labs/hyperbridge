import { Turnkey } from "@turnkey/sdk-server"
import { createAccount } from "@turnkey/viem"
import type { TurnkeySignerConfig, Signer } from "../types"
import { viemSigner } from "./viem"

/**
 * Signs through Turnkey. Its viem account already implements everything the
 * interface asks for structurally — transactions and EIP-7702 authorizations
 * both, so the policy engine sees the tuple rather than a digest — which leaves
 * this as the adapter plus a name for the logs.
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

	return { ...viemSigner(account), mode: "turnkey" }
}
