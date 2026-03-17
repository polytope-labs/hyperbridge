import type { HexString } from "@hyperbridge/sdk"
import type { Account } from "viem/accounts"

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
}

export interface SimplexSigner {
	mode: "privateKey" | "mpcVault"
	account: Account
	signBidMessage: (messageHash: HexString, chainId: number) => Promise<HexString>
}
