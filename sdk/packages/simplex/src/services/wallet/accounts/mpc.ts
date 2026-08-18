import type { HexString } from "@hyperbridge/sdk"
import { createMpcVaultAccount } from "../mpcvault"
import type { MpcVaultSignerConfig, Signer } from "../types"

/**
 * Signs through MPCVault's signing ceremony. Not a viem account underneath — the
 * API takes structured requests and digests, not viem's shapes — so this maps the
 * interface onto the service directly. The one EIP-7702 quirk (the structured
 * send cannot carry an authorization list) is handled inside the viem account
 * `createMpcVaultAccount` builds, so delegation needs nothing special here.
 */
export function mpcVaultSigner(config: MpcVaultSignerConfig): Signer {
	const { account, service } = createMpcVaultAccount(config)
	return {
		mode: "mpcVault",
		account,
		signRawHash: (hash: HexString) => service.signRawHashComponents(hash),
		signTypedData: (typedData: unknown, chainId?: number) =>
			service.signTypedData(
				JSON.stringify(typedData, (_k, v) => (typeof v === "bigint" ? v.toString() : v)),
				chainId ?? 1,
			),
	}
}
