/**
 * Wallet types aligned with `@hyperbridge/sdk` `SigningAccount`.
 * Keep `viem` on one workspace version (`sdk/package.json` -> `pnpm.overrides`) so `account` matches simplex's `viem` types.
 */
import type { HexString, SigningAccount as SdkSigningAccount } from "@hyperbridge/sdk"
import type { Account, Address } from "viem/accounts"
import type { Chain, PublicClient, Transport, WalletClient } from "viem"

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

/** EIP-7702 authorization tuple used for set-code (delegation) transactions. */
export interface Eip7702Authorization {
	chainId: number
	address: HexString
	nonce: number
	r: HexString
	s: HexString
	yParity: number
}

export interface Eip7702DelegationTxArgs {
	walletClient: WalletClient<Transport, Chain, Account>
	publicClient: PublicClient
	authorityAddress: Address
	authorization: Eip7702Authorization
	/** When `prepareTransactionRequest` omits `chainId` (MPC raw-sign path). */
	chainIdFallback: number
	gasFloor: bigint
}

export interface SigningAccount extends SdkSigningAccount {
	account: Account
	mode: "privateKey" | "mpcVault" | "turnkey"
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
	 * of an opaque digest. Absent on backends without structured 7702 support.
	 */
	signAuthorization?: (auth: {
		chainId: number
		contractAddress: HexString
		nonce: number
	}) => Promise<{ r: HexString; s: HexString; yParity: number }>
	sendEip7702DelegationTransaction: (args: Eip7702DelegationTxArgs) => Promise<HexString>
}
