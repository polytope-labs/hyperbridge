import type { HexString } from "@hyperbridge/sdk"
import { concatHex, keccak256, padHex, toHex } from "viem"
import * as grpc from "@grpc/grpc-js"
import {
	PlatformAPIClient,
	ECDSAHashFunction,
	EVMMessage_Type,
	type CreateSigningRequestRequest,
	type CreateSigningRequestResponse,
	type ExecuteSigningRequestsRequest,
	type ExecuteSigningRequestsResponse,
	type SignatureContainer_ECDSASignature,
} from "../../proto/mpcvault/platform/v1/api"
import { getLogger } from "../Logger"
import type { MpcVaultClientConfig } from "./types"

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const EXECUTE_RETRY_ATTEMPTS = 2
const EXECUTE_RETRY_BASE_DELAY_MS = 300

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

function isServiceError(err: unknown): err is grpc.ServiceError {
	return typeof (err as grpc.ServiceError | undefined)?.code === "number"
}

/** Names the RPC, the signing request and MPCVault's x-request-id, so a failure is attributable from the order log alone. */
function rpcFailure(rpc: string, err: unknown, uuid?: string): Error {
	const context: string[] = []
	if (uuid) context.push(`signing request ${uuid}`)
	if (isServiceError(err)) {
		const requestId = err.metadata?.get("x-request-id")?.[0]
		if (requestId) context.push(`x-request-id ${requestId}`)
	}
	const detail = err instanceof Error ? err.message : String(err)
	const failure = new Error(`MPCVault ${rpc} failed${context.length ? ` (${context.join(", ")})` : ""}: ${detail}`)
	;(failure as { cause?: unknown }).cause = err
	return failure
}

type MpcVaultApiError = NonNullable<CreateSigningRequestResponse["error"]>

// MPCVault reports failures with an empty message and only a code set, so the
// message alone cannot be the error signal.
function apiErrorText(error: MpcVaultApiError): string | undefined {
	if (error.message) return error.message
	if (error.serviceErrorCode !== undefined) return `service error code ${error.serviceErrorCode}`
	if (error.executeSigningRequestsErrorCode !== undefined) {
		return `execute error code ${error.executeSigningRequestsErrorCode}`
	}
	return undefined
}

function promisifyUnary<TReq, TRes>(
	method: (
		request: TReq,
		metadata: grpc.Metadata,
		callback: (error: grpc.ServiceError | null, response: TRes) => void,
	) => grpc.ClientUnaryCall,
	request: TReq,
	metadata: grpc.Metadata,
): Promise<TRes> {
	return new Promise<TRes>((resolve, reject) => {
		method(request, metadata, (err, response) => {
			if (err) reject(err)
			else resolve(response)
		})
	})
}

// ---------------------------------------------------------------------------
// MpcVaultService — gRPC with generated types
// ---------------------------------------------------------------------------

export class MpcVaultService {
	private readonly vaultUuid: string
	private readonly accountAddress: HexString
	private readonly callbackClientSignerPublicKey: string
	private readonly client: PlatformAPIClient
	private readonly metadata: grpc.Metadata
	private readonly logger = getLogger("mpc-vault")

	constructor(config: MpcVaultClientConfig) {
		this.vaultUuid = config.vaultUuid
		this.accountAddress = config.accountAddress
		this.callbackClientSignerPublicKey = config.callbackClientSignerPublicKey

		const grpcTarget = config.grpcTarget ?? "api.mpcvault.com:443"
		this.client = new PlatformAPIClient(grpcTarget, grpc.credentials.createSsl())

		this.metadata = new grpc.Metadata()
		this.metadata.add("x-mtoken", config.apiToken)
	}

	getAccountAddress(): HexString {
		return this.accountAddress
	}

	/** Close the underlying gRPC channel. Call during graceful shutdown. */
	close(): void {
		this.client.close()
	}

	// -----------------------------------------------------------------------
	// Signing request lifecycle
	// -----------------------------------------------------------------------

	private async createSigningRequest(request: CreateSigningRequestRequest): Promise<string> {
		let response: CreateSigningRequestResponse
		try {
			response = await promisifyUnary<CreateSigningRequestRequest, CreateSigningRequestResponse>(
				this.client.createSigningRequest.bind(this.client),
				request,
				this.metadata,
			)
		} catch (err) {
			throw rpcFailure("createSigningRequest", err)
		}

		const errorText = response.error && apiErrorText(response.error)
		if (errorText) {
			throw new Error(`MPCVault createSigningRequest error: ${errorText}`)
		}

		const uuid = response.signingRequest?.uuid
		if (!uuid) {
			throw new Error("MPCVault createSigningRequest response did not include signing request uuid")
		}
		this.logger.debug({ uuid }, "MPCVault signing request created")
		return uuid
	}

	// MPCVault intermittently rejects a uuid its own create just returned
	// (INVALID_ARGUMENT "Invalid uuid"), before the callback co-signer is
	// contacted. Those codes mean the server did nothing, so re-sending is safe
	// and a short retry absorbs the lag.
	private async executeSigningRequest(uuid: string): Promise<ExecuteSigningRequestsResponse> {
		for (let attempt = 0; ; attempt++) {
			let response: ExecuteSigningRequestsResponse
			try {
				response = await promisifyUnary<ExecuteSigningRequestsRequest, ExecuteSigningRequestsResponse>(
					this.client.executeSigningRequests.bind(this.client),
					{ uuid },
					this.metadata,
				)
			} catch (err) {
				const retryable =
					isServiceError(err) &&
					(err.code === grpc.status.INVALID_ARGUMENT || err.code === grpc.status.NOT_FOUND)
				if (retryable && attempt < EXECUTE_RETRY_ATTEMPTS) {
					this.logger.warn(
						{ uuid, attempt: attempt + 1, code: err.code, details: err.details },
						"MPCVault rejected a just-created signing request; retrying",
					)
					await delay(EXECUTE_RETRY_BASE_DELAY_MS * (attempt + 1))
					continue
				}
				throw rpcFailure("executeSigningRequests", err, uuid)
			}

			const errorText = response.error && apiErrorText(response.error)
			if (errorText) {
				throw new Error(`MPCVault executeSigningRequests error: ${errorText}`)
			}
			return response
		}
	}

	// -----------------------------------------------------------------------
	// Signature assembly
	// -----------------------------------------------------------------------

	private signatureFromParts(parts: SignatureContainer_ECDSASignature, normalizeV = false): HexString {
		const r = padHex(toHex(BigInt(parts.R || "0")), { size: 32 })
		const s = padHex(toHex(BigInt(parts.S || "0")), { size: 32 })
		const vInt = this.parseFlexibleBigInt(parts.V || "0")
		let v = padHex(toHex(vInt), { size: 1 })

		if (normalizeV) {
			v = padHex(toHex(vInt < 27n ? vInt + 27n : vInt), { size: 1 })
		}

		return concatHex([r, s, v]) as HexString
	}

	private ecdsaPartsToSignatureComponents(parts: SignatureContainer_ECDSASignature): {
		r: HexString
		s: HexString
		yParity: number
	} {
		const r = padHex(toHex(BigInt(parts.R || "0")), { size: 32 }) as HexString
		const s = padHex(toHex(BigInt(parts.S || "0")), { size: 32 }) as HexString
		const vInt = this.parseFlexibleBigInt(parts.V || "0")
		const yParity = vInt >= 27n ? Number(vInt - 27n) : Number(vInt)
		if (yParity !== 0 && yParity !== 1) {
			throw new Error(`Invalid signature v/yParity value: ${vInt}`)
		}
		return { r, s, yParity }
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

	private extractEcdsaSignature(
		response: ExecuteSigningRequestsResponse,
		context: string,
	): SignatureContainer_ECDSASignature {
		const parts = response.signatures?.signatures?.[0]?.ecdsaSignature
		if (!parts) {
			throw new Error(`MPCVault did not return ECDSA signature parts for ${context}`)
		}
		return parts
	}

	// -----------------------------------------------------------------------
	// Public signing methods
	// -----------------------------------------------------------------------

	/**
	 * Sign a message with EIP-191 personal_sign semantics.
	 * We apply the EIP-191 prefix ourselves and use raw_message (USE_MESSAGE_DIRECTLY)
	 * to avoid ambiguity about how MPC Vault's TYPE_PERSONAL_SIGN handles the content field.
	 */
	async signPersonalMessage(messageHash: HexString, _chainId: number): Promise<HexString> {
		const messageBytes = Buffer.from(messageHash.slice(2), "hex")
		const prefix = Buffer.from(`\x19Ethereum Signed Message:\n${messageBytes.length}`)
		const ethSignedMessageHash = keccak256(concatHex([toHex(prefix), messageHash]))
		const ethSignedBytes = Buffer.from(ethSignedMessageHash.slice(2), "hex")

		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			notes: "",
			broadcastTx: false,
			rawMessage: {
				from: this.accountAddress,
				content: ethSignedBytes,
				ecdsaHashFunction: ECDSAHashFunction.ECDSA_HASH_FUNCTION_USE_MESSAGE_DIRECTLY,
			},
		})

		const result = await this.executeSigningRequest(uuid)
		return this.signatureFromParts(this.extractEcdsaSignature(result, "personal_sign"), true)
	}

	async signTypedData(typedDataJson: string, chainId: number): Promise<HexString> {
		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			notes: "",
			broadcastTx: false,
			evmMessage: {
				chainId: String(chainId),
				from: this.accountAddress,
				type: EVMMessage_Type.TYPE_SIGN_TYPED_DATA,
				content: Buffer.from(typedDataJson),
			},
		})

		const result = await this.executeSigningRequest(uuid)
		return this.signatureFromParts(this.extractEcdsaSignature(result, "typed data signing"), true)
	}

	private async executeRawMessageSigningRequest(hash: HexString): Promise<SignatureContainer_ECDSASignature> {
		const uuid = await this.createSigningRequest({
			vaultUuid: this.vaultUuid,
			callbackClientSignerPublicKey: this.callbackClientSignerPublicKey,
			notes: "",
			broadcastTx: false,
			rawMessage: {
				from: this.accountAddress,
				content: Buffer.from(hash.slice(2), "hex"),
				ecdsaHashFunction: ECDSAHashFunction.ECDSA_HASH_FUNCTION_USE_MESSAGE_DIRECTLY,
			},
		})

		const result = await this.executeSigningRequest(uuid)
		return this.extractEcdsaSignature(result, "raw_message")
	}

	async signRawHash(hash: HexString): Promise<HexString> {
		const parts = await this.executeRawMessageSigningRequest(hash)
		return this.signatureFromParts(parts, false)
	}

	async signRawHashComponents(hash: HexString): Promise<{ r: HexString; s: HexString; yParity: number }> {
		const parts = await this.executeRawMessageSigningRequest(hash)
		return this.ecdsaPartsToSignatureComponents(parts)
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
			notes: "",
			broadcastTx: false,
			evmSendCustom: {
				chainId: String(params.chainId),
				from: this.accountAddress,
				to: params.to ?? "",
				value: (params.value ?? 0n).toString(),
				input: params.data && params.data !== "0x" ? Buffer.from(params.data.slice(2), "hex") : Buffer.alloc(0),
				gasFee: {
					gasLimit: params.gasLimit?.toString(),
					maxFee: params.maxFeePerGas?.toString(),
					maxPriorityFee: params.maxPriorityFeePerGas?.toString(),
				},
				nonce: params.nonce?.toString(),
			},
		})

		const result = await this.executeSigningRequest(uuid)
		if (!result.signedTransaction) {
			throw new Error("MPCVault did not return signed transaction")
		}
		return this.normalizeHex(result.signedTransaction)
	}
}
