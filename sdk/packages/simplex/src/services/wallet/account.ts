import type { HexString, SigningAccount } from "@hyperbridge/sdk"
import {
	hashTypedData,
	keccak256,
	serializeSignature,
	serializeTransaction,
	type TransactionSerializable,
} from "viem"
import { hashAuthorization } from "viem/utils"
import { toAccount, type LocalAccount } from "viem/accounts"
import type {
	AuthorizationRequest,
	Eip7702Authorization,
	Signature,
	Signer,
	SignerTransaction,
	TypedDataPayload,
} from "./types"

/**
 * Builds the viem account simplex's wallet clients run on from a {@link Signer}.
 *
 * This is the whole reason `Signer` can stay viem-free: the mapping lives here,
 * on our side of the boundary, instead of in every consumer's signer.
 */
export function accountFor(signer: Signer): LocalAccount {
	return toAccount({
		address: signer.address,
		signTypedData: (typedData) => signer.signTypedData(typedData as never) as Promise<`0x${string}`>,
		signTransaction: async (transaction) =>
			(await signer.signTransaction(toSignerTransaction(transaction as TransactionSerializable))) as `0x${string}`,
		// viem's `toAccount` requires it and no solver path personal-signs: bids are
		// EIP-712, delegations are authorization tuples.
		signMessage: async () => {
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
 * Builds a {@link Signer} from a backend that only signs 32-byte digests — an
 * HSM, a KMS, a remote signing service.
 *
 * The three operations are hashed and serialised here, so nothing outside this
 * package has to know how an EIP-7702 authorization is hashed or how an EIP-1559
 * transaction is encoded. A backend that *can* see structure should implement
 * {@link Signer} directly instead, so its policy engine keeps that visibility.
 *
 * ```ts
 * const signer = digestSigner({
 *   address: await hsm.address(),
 *   mode: "hsm",
 *   sign: (hash) => hsm.sign(hash),
 * })
 * ```
 */
export function digestSigner(opts: {
	address: HexString
	mode: string
	sign(hash: HexString): Promise<Signature>
}): Signer {
	// Guard the backend's signature once, for every operation. Most HSM/KMS docs
	// hand back the pre-EIP-1559 v (27/28), and viem encodes any truthy yParity
	// as parity 1 when serialising — so an unguarded 27 yields signatures that
	// recover to a stranger. For EIP-7702 that failure is silent: an invalid
	// authorization is skipped without reverting, the receipt reads success, and
	// the solver runs undelegated.
	const sign = async (hash: HexString): Promise<Signature> => {
		const signature = await opts.sign(hash)
		if (signature.yParity !== 0 && signature.yParity !== 1) {
			throw new Error(
				`digestSigner: sign() returned yParity ${signature.yParity}; expected 0 or 1 ` +
					"(a backend returning the legacy v should subtract 27)",
			)
		}
		return signature
	}

	return {
		address: opts.address,
		mode: opts.mode,
		signTypedData: async (typedData: TypedDataPayload) =>
			serializeSignature(await sign(hashTypedData(typedData as never) as HexString)) as HexString,
		signAuthorization: (auth: AuthorizationRequest) =>
			sign(
				hashAuthorization({
					address: auth.contractAddress,
					chainId: auth.chainId,
					nonce: auth.nonce,
				}) as HexString,
			),
		signTransaction: async (tx: SignerTransaction) => {
			const serializable = tx as unknown as TransactionSerializable
			const signature = await sign(keccak256(serializeTransaction(serializable)) as HexString)
			return serializeTransaction(serializable, signature) as HexString
		},
	}
}

/**
 * The sdk's `SigningAccount`, backed by a {@link Signer}.
 *
 * The two describe the same operation; only the typed-data parameter differs —
 * the sdk takes `unknown` so it depends on no payload type of ours, and `Signer`
 * takes {@link TypedDataPayload} so a `domain.chainId` a backend needs is visible
 * in the type. The cast is that difference and nothing more.
 */
export function sdkSigningAccount(signer: Signer): SigningAccount {
	return { signTypedData: (typedData) => signer.signTypedData(typedData as TypedDataPayload) }
}
