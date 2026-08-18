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

/** A secp256k1 signature in split form, which is how EIP-7702 and raw digests consume it. */
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
 * An address and two methods are required:
 *
 * - `signTypedData` — EIP-712. The hot path: every bid UserOperation and every
 *   EIP-2612 permit the paymaster needs.
 * - `signRawHash` — raw ECDSA over a digest. Signs EIP-7702 authorizations, and
 *   transactions too unless the backend implements `signTransaction`.
 *
 * The two optional methods exist for backends whose policy engine wants to see
 * what it is authorising rather than an opaque digest. Omit them and simplex
 * serialises the payload itself and asks for a digest signature.
 */
export interface Signer {
	/** The address every fill, bid and delegation is attributed to. */
	readonly address: HexString
	signTypedData(typedData: TypedDataPayload): Promise<HexString>
	signRawHash(hash: HexString): Promise<Signature>

	/**
	 * Names the backend in logs. Free-form — the built-ins report `"privateKey"`,
	 * `"mpcVault"` and `"turnkey"`; a signer that sets nothing logs as `"custom"`.
	 */
	readonly mode?: string
	/**
	 * Signs an EIP-7702 authorization tuple natively, where the backend has a
	 * structured encoding for it (e.g. Turnkey's PAYLOAD_ENCODING_EIP7702_AUTHORIZATION).
	 * Preferred over `signRawHash(authHash)` because the backend sees
	 * (chainId, delegate, nonce) instead of an opaque digest.
	 */
	signAuthorization?(auth: { chainId: number; contractAddress: HexString; nonce: number }): Promise<Signature>
	/**
	 * Signs and serialises a transaction, where the backend takes transactions
	 * rather than digests. Returns the signed, RLP-encoded transaction, ready to
	 * broadcast. Omit it and simplex serialises the transaction and signs the
	 * digest with `signRawHash`.
	 */
	signTransaction?(tx: SignerTransaction): Promise<HexString>
}
