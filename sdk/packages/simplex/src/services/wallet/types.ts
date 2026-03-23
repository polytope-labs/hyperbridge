import type { HexString, SigningAccount as SdkSigningAccount } from "@hyperbridge/sdk"

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
}

export type SignerConfig =
	| {
			type: SignerType.PrivateKey
			privateKey: HexString
	  }
	| {
			type: SignerType.MpcVault
			mpcVault: MpcVaultSignerConfig
	  }

export interface SigningAccount extends SdkSigningAccount {
	mode: "privateKey" | "mpcVault"
}
