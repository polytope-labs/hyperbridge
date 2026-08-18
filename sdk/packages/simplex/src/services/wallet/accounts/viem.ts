import type { HexString } from "@hyperbridge/sdk"
import { parseSignature } from "viem"
import type { LocalAccount } from "viem/accounts"
import type { Signature, Signer } from "../types"

/** viem's serialised signature in the split form {@link Signer} works in. */
export function splitSignature(signature: HexString, source: string): Signature {
	const sig = parseSignature(signature)
	const yParity = sig.yParity ?? (sig.v !== undefined ? Number(sig.v >= 27n ? sig.v - 27n : sig.v) : undefined)
	if (yParity !== 0 && yParity !== 1) {
		throw new Error(`Failed to derive yParity from ${source} signature`)
	}
	return { r: sig.r as HexString, s: sig.s as HexString, yParity }
}

/**
 * Adapts a viem local account into a {@link Signer}.
 *
 * This is the path for custody that already speaks viem — a private key, a
 * `toAccount` wrapper around an HSM or a remote signing service, or a provider
 * SDK that hands you an account. Everything the account can do structurally
 * (transactions, EIP-7702 authorizations) is carried through rather than
 * flattened into digests.
 *
 * The account must expose `sign`: EIP-7702 delegation needs a raw digest
 * signature, and a solver that cannot delegate cannot take part in solver
 * selection, so its absence is rejected here rather than at the first fill.
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
		address: account.address as HexString,
		mode: source,
		signTypedData: (typedData) => account.signTypedData(typedData as never) as Promise<HexString>,
		signRawHash: async (hash) => splitSignature((await sign({ hash })) as HexString, source),
		signTransaction: (tx) => account.signTransaction(tx as never) as Promise<HexString>,
		...(account.signAuthorization
			? {
					signAuthorization: async (auth: { chainId: number; contractAddress: HexString; nonce: number }) => {
						const signed = await account.signAuthorization!(auth)
						const yParity =
							signed.yParity ?? (signed.v !== undefined ? Number(signed.v >= 27n ? signed.v - 27n : signed.v) : undefined)
						if (yParity !== 0 && yParity !== 1) {
							throw new Error(`Failed to derive yParity from ${source} authorization signature`)
						}
						return { r: signed.r as HexString, s: signed.s as HexString, yParity }
					},
				}
			: {}),
	}
}
