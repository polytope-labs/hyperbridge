import type { HexString } from "@hyperbridge/sdk"
import { keccak256, serializeSignature, serializeTransaction, type TransactionSerializable } from "viem"
import { toAccount, type LocalAccount } from "viem/accounts"
import type { SigningAccount } from "@hyperbridge/sdk"
import type { Eip7702Authorization, Signer, SignerTransaction, TypedDataPayload } from "./types"

/**
 * Builds the viem account simplex's wallet clients run on from a {@link Signer}.
 *
 * This is the whole reason `Signer` can stay viem-free: the mapping lives here,
 * on our side of the boundary, instead of in every consumer's signer. A backend
 * that takes transactions implements `signTransaction` and viem's prepared
 * request is mapped onto our shape; one that takes digests implements nothing
 * and the transaction is serialised here and signed as a hash.
 */
export function accountFor(signer: Signer): LocalAccount {
	return toAccount({
		address: signer.address,
		sign: async ({ hash }) => serializeSignature(await signer.signRawHash(hash as HexString)),
		signTypedData: (typedData) => signer.signTypedData(typedData as never) as Promise<`0x${string}`>,
		signTransaction: async (transaction, options) => {
			const serializer = options?.serializer ?? serializeTransaction
			const tx = transaction as TransactionSerializable
			if (signer.signTransaction) return signer.signTransaction(toSignerTransaction(tx))
			const unsigned = serializer(tx) as HexString
			const signature = await signer.signRawHash(keccak256(unsigned) as HexString)
			return serializer(tx, signature)
		},
		// viem's `toAccount` requires it and no solver path personal-signs, so this
		// is unreachable rather than unimplemented: bids are EIP-712, and raw
		// digests go through `signRawHash`.
		signMessage: () => {
			throw new Error("Simplex never signs personal messages")
		},
	})
}

/** viem's prepared transaction request, narrowed to what a signing backend is given. */
function toSignerTransaction(tx: TransactionSerializable): SignerTransaction {
	const eip7702 = "authorizationList" in tx ? tx.authorizationList : undefined
	// viem fills `chainId` in `prepareTransactionRequest`; a transaction that
	// reached a signer without one would be replayable on every chain.
	if (tx.chainId === undefined) throw new Error("Cannot sign a transaction with no chainId")
	return {
		chainId: tx.chainId,
		type: tx.type,
		to: (tx.to ?? undefined) as HexString | undefined,
		value: tx.value,
		data: tx.data as HexString | undefined,
		nonce: tx.nonce,
		gas: tx.gas,
		maxFeePerGas: "maxFeePerGas" in tx ? tx.maxFeePerGas : undefined,
		maxPriorityFeePerGas: "maxPriorityFeePerGas" in tx ? tx.maxPriorityFeePerGas : undefined,
		authorizationList: eip7702 as Eip7702Authorization[] | undefined,
	}
}

/**
 * The sdk's `SigningAccount`, backed by a {@link Signer}.
 *
 * The two describe the same operations; only the typed-data parameter differs —
 * the sdk takes `unknown` so it depends on no payload type of ours, and `Signer`
 * takes {@link TypedDataPayload} so a `domain.chainId` a backend needs is visible
 * in the type. The cast is that difference and nothing more.
 */
export function sdkSigningAccount(signer: Signer): SigningAccount {
	return {
		signRawHash: (hash) => signer.signRawHash(hash),
		signTypedData: (typedData) => signer.signTypedData(typedData as TypedDataPayload),
	}
}
