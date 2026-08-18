import type { HexString } from "@hyperbridge/sdk"
import { parseSignature } from "viem"
import type { LocalAccount } from "viem/accounts"
import type { Signer } from "../types"

/**
 * The split components a raw-hash signature is consumed as, from viem's
 * serialised form. Shared by every backend that signs through a viem account.
 */
export function splitSignature(signature: HexString, source: string): { r: HexString; s: HexString; yParity: number } {
	const sig = parseSignature(signature)
	const yParity = sig.yParity ?? (sig.v !== undefined ? Number(sig.v >= 27n ? sig.v - 27n : sig.v) : undefined)
	if (yParity !== 0 && yParity !== 1) {
		throw new Error(`Failed to derive yParity from ${source} signature`)
	}
	return { r: sig.r as HexString, s: sig.s as HexString, yParity }
}

/**
 * Turns a viem local account into a {@link Signer}.
 *
 * This is the adapter for custody that already speaks viem — a private key, a
 * `toAccount` wrapper around an HSM or a remote signing service, or a provider
 * SDK that hands you an account. The account must expose `sign` (raw digest
 * signing): EIP-7702 delegation is signed with it, and a solver that cannot
 * delegate cannot take part in solver selection, so its absence is rejected here
 * rather than at the first fill.
 *
 * Backends with a structured EIP-7702 path should spread the result and add
 * `signAuthorization`, the way `turnkeySigner` does.
 */
export function viemSigner(account: LocalAccount): Signer {
	const sign = account.sign
	if (!sign) {
		throw new Error(
			"Signer account cannot sign raw hashes: EIP-7702 delegation needs `account.sign`. " +
				"Pass a `sign` implementation to viem's `toAccount`, or implement the Signer interface directly.",
		)
	}
	const source = account.source || "custom"

	return {
		mode: source,
		account,
		signTypedData: (typedData: unknown, _chainId?: number) =>
			account.signTypedData(typedData as Parameters<LocalAccount["signTypedData"]>[0]) as Promise<HexString>,
		signRawHash: async (hash: HexString) => splitSignature((await sign({ hash })) as HexString, source),
	}
}

