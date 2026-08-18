import type { HexString } from "@hyperbridge/sdk"
import { parseSignature } from "viem"
import { hashAuthorization } from "viem/utils"
import type { LocalAccount } from "viem/accounts"
import type { AuthorizationRequest, Signature, Signer } from "../types"

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
 * SDK that hands you an account. Transactions go through the account's own
 * `signTransaction`, so a backend that inspects them keeps doing so.
 *
 * EIP-7702 is the one operation viem makes optional: accounts that implement
 * `signAuthorization` (a private key, Turnkey) use it, and the rest fall back to
 * signing the authorization digest with `sign`. An account with neither cannot
 * delegate, and a solver that cannot delegate cannot take part in solver
 * selection, so that is rejected here rather than at the first fill.
 */
export function viemSigner(account: LocalAccount): Signer {
	const source = account.source || "custom"
	const signAuthorization = authorizationSignerFor(account, source)

	return {
		address: account.address as HexString,
		mode: source,
		signTypedData: (typedData) => account.signTypedData(typedData as never) as Promise<HexString>,
		signTransaction: (tx) => account.signTransaction(tx as never) as Promise<HexString>,
		signAuthorization,
	}
}

function authorizationSignerFor(
	account: LocalAccount,
	source: string,
): (auth: AuthorizationRequest) => Promise<Signature> {
	if (account.signAuthorization) {
		return async (auth) => {
			const signed = await account.signAuthorization!({
				address: auth.contractAddress,
				chainId: auth.chainId,
				nonce: auth.nonce,
			})
			const yParity =
				signed.yParity ?? (signed.v !== undefined ? Number(signed.v >= 27n ? signed.v - 27n : signed.v) : undefined)
			if (yParity !== 0 && yParity !== 1) {
				throw new Error(`Failed to derive yParity from ${source} authorization signature`)
			}
			return { r: signed.r as HexString, s: signed.s as HexString, yParity }
		}
	}

	const sign = account.sign
	if (!sign) {
		throw new Error(
			"Signer account cannot sign EIP-7702 authorizations: the account implements neither " +
				"`signAuthorization` nor `sign`, so this solver could never delegate. Pass one of them to viem's " +
				"`toAccount`, or implement the Signer interface directly.",
		)
	}
	return async (auth) =>
		splitSignature(
			(await sign({
				hash: hashAuthorization({ address: auth.contractAddress, chainId: auth.chainId, nonce: auth.nonce }),
			})) as HexString,
			`${source} authorization`,
		)
}
