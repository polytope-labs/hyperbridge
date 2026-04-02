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
	sendEip7702DelegationTransaction: (args: Eip7702DelegationTxArgs) => Promise<HexString>
}
