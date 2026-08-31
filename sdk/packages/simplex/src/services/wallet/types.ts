/**
 * The signing contract a solver runs on, and the config shapes the bundled
 * implementations take.
 *
 * `Signer` is the whole surface: implement it and Simplex signs with your
 * backend. Nothing here is viem-shaped, so a consumer never has to match this
 * package's viem version to satisfy it — `viemSigner` adapts a viem account into
 * one, and simplex builds the account its wallet clients need from whatever it
 * is given.
 */
import type { HexString } from "@hyperbridge/sdk"

export interface MpcVaultClientConfig {
	apiToken: string
	vaultUuid: string
	accountAddress: HexString
	callbackClientSignerPublicKey: string
	/**
	 * gRPC target address. Defaults to "api.mpcvault.com:443".
	 * Replaces the previous REST `baseUrl` field.
	 */
	grpcTarget?: string
}

export interface MpcVaultSignerConfig {
	apiToken: string
	vaultUuid: string
	accountAddress: HexString
	callbackClientSignerPublicKey: string
	/**
	 * gRPC target address. Defaults to "api.mpcvault.com:443".
	 * Replaces the previous REST `baseUrl` field.
	 */
	grpcTarget?: string
}

export enum SignerType {
	PrivateKey = "privateKey",
	MpcVault = "mpcVault",
	Turnkey = "turnkey",
}

export interface TurnkeySignerConfig {
	organizationId: string
	apiPublicKey: string
	apiPrivateKey: string
	signWith: string
}

export interface PrivateKeySignerConfig {
	key: HexString
}

/**
 * The `[simplex.signer]` block of the TOML file the binary reads — a tagged
 * union over the bundled backends. `createSigner` turns one into a {@link Signer};
 * `Simplex.start` takes the `Signer`, not this.
 */
export type SignerConfig =
	| ({
			type: SignerType.PrivateKey
	  } & PrivateKeySignerConfig)
	| ({
			type: SignerType.MpcVault
	  } & MpcVaultSignerConfig)
	| ({
			type: SignerType.Turnkey
	  } & TurnkeySignerConfig)

/**
 * A secp256k1 signature in split form, which is how EIP-7702 and raw digests
 * consume it: `r` and `s` as 32-byte 0x-hex, and `yParity` strictly 0 or 1.
 * A backend that hands back the legacy `v` (27/28) should subtract 27 — the
 * delegation path rejects anything else rather than letting viem mis-encode it.
 */
export interface Signature {
	r: HexString
	s: HexString
	yParity: number
}

/**
 * An EIP-712 payload, as `eth_signTypedData_v4` defines it.
 *
 * `domain.chainId` is the only field a signing backend is likely to read for
 * itself: the digest covers it, and backends that scope a signing request to a
 * chain (MPCVault) take it from here rather than from a separate argument.
 *
 * **List `EIP712Domain` in `types` on any payload you build.** viem ignores it
 * when hashing locally, so a payload without it looks correct against a private
 * key. Backends that hash server-side from the JSON derive the domain type from
 * that entry — MPCVault and Turnkey both do — and without it they sign a
 * different digest than the one your verifier checks. The failure is quiet: the
 * signature is well-formed and recovers to an address that is not yours.
 * `CryptoUtils.packedUserOpTypedData` in the sdk sets it for this reason.
 */
export interface TypedDataPayload {
	domain?: {
		name?: string
		version?: string
		chainId?: number | bigint
		verifyingContract?: HexString
		salt?: HexString
	}
	types: Record<string, readonly { name: string; type: string }[]>
	primaryType: string
	message: Record<string, unknown>
}

/** EIP-7702 authorization tuple, signed, as a set-code transaction carries it. */
export interface Eip7702Authorization extends Signature {
	chainId: number
	address: HexString
	nonce: number
}

/**
 * A transaction to sign, handed to backends that implement
 * {@link Signer.signTransaction}. Field names follow the JSON-RPC transaction
 * object; simplex only ever sends EIP-1559 transfers and EIP-7702 set-code
 * transactions, so those are the fields modelled here.
 */
export interface SignerTransaction {
	chainId: number
	type?: string
	to?: HexString
	value?: bigint
	data?: HexString
	nonce?: number
	gas?: bigint
	maxFeePerGas?: bigint
	maxPriorityFeePerGas?: bigint
	/** Present on EIP-7702 set-code transactions, and on nothing else. */
	authorizationList?: Eip7702Authorization[]
}

/**
 * Everything a solver asks of the key that owns its funds. Pass an
 * implementation to `Simplex.start({ signer })`.
 *
 * Three operations, one identity. Each operation names what is being authorised
 * — a typed-data payload, an EIP-7702 authorization, a transaction — rather than
 * a digest, so a custody backend's policy engine can read what it is signing.
 *
 * A backend that only signs digests does not implement this by hand: pass its
 * `sign(hash)` to {@link digestSigner}, which does the hashing and serialising
 * and returns a `Signer`.
 */
export interface Signer {
	/** The address every fill, bid and delegation is attributed to. */
	readonly address: HexString
	/** Names the backend in logs — `"privateKey"`, `"turnkey"`, or whatever names yours. */
	readonly mode: string
	/**
	 * EIP-712. Signs every bid UserOperation and every paymaster permit.
	 *
	 * Returns the 65-byte `r ‖ s ‖ v` signature as 0x-hex (132 characters), with
	 * `v` 27 or 28 — exactly what `eth_signTypedData_v4` and viem's
	 * `signTypedData` produce. On-chain validation recovers against this form.
	 */
	signTypedData(typedData: TypedDataPayload): Promise<HexString>
	/**
	 * Signs an EIP-7702 authorization tuple — the delegation that makes this
	 * account a solver. Backends with a structured encoding for it (Turnkey) use
	 * that; others sign `keccak256(0x05 ‖ rlp([chainId, contractAddress, nonce]))`.
	 *
	 * Returns the SPLIT signature — see {@link Signature}: `yParity` must be 0 or
	 * 1, not the legacy `v`. This is the one method that does not return hex,
	 * because the authorization is embedded field-by-field in the transaction.
	 */
	signAuthorization(auth: AuthorizationRequest): Promise<Signature>
	/**
	 * Signs and serialises a transaction. Backends that take transactions
	 * (MPCVault's vault API, Turnkey's transaction payloads) keep the transaction
	 * legible to their policy engine; others serialise it and sign the digest.
	 *
	 * Returns the WHOLE SIGNED TRANSACTION — typed-envelope RLP as
	 * `eth_sendRawTransaction` takes it (`0x02…` for EIP-1559, `0x04…` for
	 * set-code), not a bare signature. viem's `serializeTransaction(tx, signature)`
	 * produces it from a signed digest.
	 */
	signTransaction(tx: SignerTransaction): Promise<HexString>
}

/** The tuple an EIP-7702 authorization signature covers. */
export interface AuthorizationRequest {
	chainId: number
	/** The contract the account delegates to. */
	contractAddress: HexString
	nonce: number
}
