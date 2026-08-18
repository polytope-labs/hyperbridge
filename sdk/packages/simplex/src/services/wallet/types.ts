/**
 * The signing contract a solver runs on, and the config shapes the bundled
 * implementations take.
 *
 * `Signer` is the whole surface: implement it and Simplex signs with your
 * backend. The built-ins (`privateKeySigner`, `turnkeySigner`, `mpcVaultSigner`,
 * `viemSigner`) are implementations of it with no privileged access — including
 * MPCVault, whose EIP-7702 workaround lives in its viem account rather than in
 * this interface.
 *
 * Keep `viem` on one workspace version (`sdk/package.json` -> `pnpm.overrides`) so `account` matches simplex's `viem` types.
 */
import type { HexString, SigningAccount as SdkSigningAccount } from "@hyperbridge/sdk"
import type { Account } from "viem/accounts"

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
 * Everything a solver asks of the key that owns its funds. Pass an
 * implementation to `Simplex.start({ signer })`.
 *
 * Three members, and the last is optional:
 *
 * - `account` — a viem `Account`. Every transaction the solver sends (the
 *   delegation tx, rebalances, operator transfers) goes out through a wallet
 *   client built on it, so it must be able to sign transactions. Wrap a remote
 *   backend with viem's `toAccount`, or hand any viem account to `viemSigner` and
 *   have the rest of this interface derived from it.
 * - `signTypedData` — EIP-712. The hot path: every bid UserOperation and every
 *   EIP-2612 permit the paymaster needs.
 * - `signRawHash` — raw ECDSA over a digest, split into components. Signs the
 *   EIP-7702 authorization when the backend has no structured path for it.
 *
 * `chainId` is passed to the signing calls for backends whose policy engine
 * scopes a signature to a chain; implementations that do not care may ignore it.
 */
export interface Signer extends SdkSigningAccount {
	/** viem account the solver's wallet clients are built on. */
	account: Account
	/**
	 * Names the backend in logs. Free-form — the built-ins report `"privateKey"`,
	 * `"mpcVault"` and `"turnkey"`; a signer that sets nothing logs as `"custom"`.
	 */
	mode?: string
	/**
	 * Signs an EIP-712 typed-data payload (e.g. an EIP-2612 USDC permit for the Circle Paymaster).
	 * The shape of `typedData` matches viem's `TypedDataDefinition`.
	 * MPC adapter must JSON.stringify before delegating to MpcVaultService.signTypedData.
	 */
	signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString>
	/**
	 * Signs an EIP-7702 authorization tuple natively when the signing backend supports
	 * a structured encoding for it (e.g. Turnkey's PAYLOAD_ENCODING_EIP7702_AUTHORIZATION).
	 * Preferred over `signRawHash(authHash)` because the backend sees the tuple instead
	 * of an opaque digest. Omit on backends without structured 7702 support — the solver
	 * falls back to signing the authorization digest with `signRawHash`.
	 */
	signAuthorization?: (auth: {
		chainId: number
		contractAddress: HexString
		nonce: number
	}) => Promise<{ r: HexString; s: HexString; yParity: number }>
}
