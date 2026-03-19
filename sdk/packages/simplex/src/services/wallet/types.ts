import type { HexString, SigningAccount as SdkSigningAccount } from "@hyperbridge/sdk"

export interface MpcVaultError {
	message?: string
}

export interface EcdsaSignatureParts {
	R?: string
	S?: string
	V?: string
}

export interface ExecuteSigningResponse {
	txHash?: string
	signedTransaction?: string
	signatures?: {
		signatures?: Array<{
			ecdsaSignature?: EcdsaSignatureParts
		}>
	}
	error?: MpcVaultError
}

export interface SigningRequestResponse {
	signingRequest?: {
		uuid?: string
	}
	error?: MpcVaultError
}

export interface MpcVaultClientConfig {
	apiToken: string
	vaultUuid: string
	accountAddress: HexString
	callbackClientSignerPublicKey: string
	baseUrl?: string
}

export interface MpcVaultSignerConfig {
	apiToken: string
	vaultUuid: string
	accountAddress: HexString
	callbackClientSignerPublicKey: string
	baseUrl?: string
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

export type CreateSigningRequestPayload = {
	vaultUuid: string
	callbackClientSignerPublicKey: string
	broadcastTx?: boolean
	notes?: string
	evmSendCustom?: {
		chainId: number
		from: HexString
		to: HexString | ""
		value: string
		input: HexString
		gasFee?: {
			gasLimit?: string
			maxFee?: string
			maxPriorityFee?: string
		}
		nonce?: number
	}
	evmMessage?: {
		chainId: string
		from: HexString
		type: "TYPE_PERSONAL_SIGN" | "TYPE_SIGN_TYPED_DATA"
		content: string
	}
	rawMessage?: {
		from: HexString
		content: HexString
		ecdsaHashFunction: "ECDSA_HASH_FUNCTION_USE_MESSAGE_DIRECTLY" | "ECDSA_HASH_FUNCTION_SHA256"
	}
}

export interface SigningAccount extends SdkSigningAccount {
	mode: "privateKey" | "mpcVault"
}
