import { type HexString } from "@hyperbridge/sdk"
import { concatHex, padHex, toHex, type Hex } from "viem"
import type {
	CreateSigningRequestPayload,
	EcdsaSignatureParts,
	ExecuteSigningResponse,
	MpcVaultClientConfig,
	SigningRequestResponse,
} from "./types"

export class MpcVaultService {
	private readonly apiToken: string
	private readonly vaultUuid: string
	private readonly accountAddress: HexString
	private readonly callbackClientSignerPublicKey: string
	private readonly baseUrl: string

	constructor(config: MpcVaultClientConfig) {
		this.apiToken = config.apiToken
		this.vaultUuid = config.vaultUuid
		this.accountAddress = config.accountAddress
		this.callbackClientSignerPublicKey = config.callbackClientSignerPublicKey
		this.baseUrl = config.baseUrl ?? "https://api.mpcvault.com"
	}

	getAccountAddress(): HexString {
		return this.accountAddress
	}

	private async post<TResponse>(path: string, body: unknown): Promise<TResponse> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			method: "POST",
			headers: {
				"content-type": "application/json",
				"x-mtoken": this.apiToken,
			},
			body: JSON.stringify(body),
		})

		if (!response.ok) {
			throw new Error(`MPCVault request failed (${response.status} ${response.statusText})`)
		}

		return (await response.json()) as TResponse
	}

	private async createSigningRequest(payload: CreateSigningRequestPayload): Promise<string> {
		const response = await this.post<SigningRequestResponse>("/v1/createSigningRequest", payload)
		if (response.error) {
			throw new Error(`MPCVault createSigningRequest error: ${response.error.message ?? "unknown error"}`)
		}
		const uuid = response.signingRequest?.uuid
		if (!uuid) {
			throw new Error("MPCVault createSigningRequest response did not include signing request uuid")
		}
		return uuid
	}

	private async executeSigningRequest(uuid: string): Promise<ExecuteSigningResponse> {
		const response = await this.post<ExecuteSigningResponse>("/v1/executeSigningRequests", { uuid })
		if (response.error) {
			throw new Error(`MPCVault executeSigningRequests error: ${response.error.message ?? "unknown error"}`)
		}
		return response
	}

	private signatureFromParts(parts: EcdsaSignatureParts): HexString {
		const r = padHex((parts.R ?? "0x0") as Hex, { size: 32 })
		const s = padHex((parts.S ?? "0x0") as Hex, { size: 32 })
		const vInt = this.parseFlexibleBigInt(parts.V ?? "0")
		const v = padHex(toHex(vInt), { size: 1 })
		return concatHex([r, s, v]) as HexString
	}

	private parseFlexibleBigInt(value: string): bigint {
		if (value.startsWith("0x") || value.startsWith("0X")) {
			return BigInt(value)
		}
		return BigInt(value)
	}

	private normalizeHex(value: string): HexString {
		return (value.startsWith("0x") ? value : `0x${value}`) as HexString
	}

	async signPersonalMessage(messageHash: HexString, chainId: number): Promise<HexString> {
		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			evmMessage: {
				chainId: String(chainId),
				from: this.accountAddress,
				type: "TYPE_PERSONAL_SIGN",
				content: messageHash,
			},
		})

		const result = await this.executeSigningRequest(uuid)
		const parts = result.signatures?.signatures?.[0]?.ecdsaSignature
		if (!parts) {
			throw new Error("MPCVault did not return ECDSA signature parts for personal_sign")
		}
		return this.signatureFromParts(parts)
	}

	async signTypedData(typedDataJson: string, chainId: number): Promise<HexString> {
		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			evmMessage: {
				chainId: String(chainId),
				from: this.accountAddress,
				type: "TYPE_SIGN_TYPED_DATA",
				content: toHex(typedDataJson),
			},
		})

		const result = await this.executeSigningRequest(uuid)
		const parts = result.signatures?.signatures?.[0]?.ecdsaSignature
		if (!parts) {
			throw new Error("MPCVault did not return ECDSA signature parts for typed data signing")
		}
		return this.signatureFromParts(parts)
	}

	async signTransaction(params: {
		chainId: number
		to?: HexString
		value?: bigint
		data?: HexString
		nonce?: number
		gasLimit?: bigint
		maxFeePerGas?: bigint
		maxPriorityFeePerGas?: bigint
	}): Promise<HexString> {
		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			broadcastTx: false,
			evmSendCustom: {
				chainId: params.chainId,
				from: this.accountAddress,
				to: params.to ?? "",
				value: (params.value ?? 0n).toString(),
				input: params.data ?? "0x",
				gasFee: {
					gasLimit: params.gasLimit?.toString(),
					maxFee: params.maxFeePerGas?.toString(),
					maxPriorityFee: params.maxPriorityFeePerGas?.toString(),
				},
				nonce: params.nonce,
			},
		})

		const result = await this.executeSigningRequest(uuid)
		if (!result.signedTransaction) {
			throw new Error("MPCVault did not return signed transaction")
		}
		return this.normalizeHex(result.signedTransaction)
	}
}
