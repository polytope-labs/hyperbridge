import type { HexString } from "@hyperbridge/sdk"
import { toAccount, privateKeyToAccount, type Account } from "viem/accounts"
import { sign } from "viem/accounts"
import { MpcVaultService } from "./mpcvault"
import { SignerType, type MpcVaultSignerConfig, type SignerConfig, type SigningAccount } from "./types"

function requireChainId(value: unknown, context: string): number {
	if (typeof value === "number" && Number.isFinite(value)) return value
	if (typeof value === "bigint") return Number(value)
	throw new Error(`Missing chainId for MPCVault ${context}`)
}

function createMpcVaultAccount(config: MpcVaultSignerConfig): { account: Account; service: MpcVaultService } {
	const service = new MpcVaultService({
		apiToken: config.apiToken,
		vaultUuid: config.vaultUuid,
		accountAddress: config.accountAddress,
		callbackClientSignerPublicKey: config.callbackClientSignerPublicKey,
		baseUrl: config.baseUrl,
	})

	const account = toAccount({
		address: config.accountAddress,
		async signMessage({ message }): Promise<HexString> {
			const raw = typeof message === "object" && "raw" in message ? (message.raw as HexString) : undefined
			if (!raw) {
				throw new Error("MPCVault signer requires message.raw for signMessage")
			}
			throw new Error(
				"MPCVault signMessage requires chain-specific context. Use signBidMessage(messageHash, chainId).",
			)
		},
		async signTransaction(transaction): Promise<HexString> {
			const params = transaction as {
				chainId?: number | bigint
				to?: HexString
				value?: bigint
				data?: HexString
				nonce?: number
				gas?: bigint
				maxFeePerGas?: bigint
				maxPriorityFeePerGas?: bigint
			}
			return service.signTransaction({
				chainId: requireChainId(params.chainId, "transaction signing"),
				to: params.to,
				value: params.value,
				data: params.data,
				nonce: params.nonce,
				gasLimit: params.gas,
				maxFeePerGas: params.maxFeePerGas,
				maxPriorityFeePerGas: params.maxPriorityFeePerGas,
			})
		},
		async signTypedData(typedDataDefinition): Promise<HexString> {
			const typedData = typedDataDefinition as {
				domain?: { chainId?: number | bigint }
			}
			const chainId = requireChainId(typedData.domain?.chainId, "typed-data signing")
			return service.signTypedData(JSON.stringify(typedDataDefinition), chainId)
		},
	})

	return { account, service }
}

function parseSignature(signature: HexString): { r: HexString; s: HexString; yParity: number } {
	const hex = signature.slice(2)
	if (hex.length !== 130) {
		throw new Error(`Invalid signature length: expected 65 bytes, got ${hex.length / 2} bytes`)
	}
	const r = `0x${hex.slice(0, 64)}` as HexString
	const s = `0x${hex.slice(64, 128)}` as HexString
	const v = Number.parseInt(hex.slice(128, 130), 16)
	const yParity = v >= 27 ? v - 27 : v
	if (yParity !== 0 && yParity !== 1) {
		throw new Error(`Invalid signature v/yParity value: ${v}`)
	}
	return { r, s, yParity }
}

export function createSimplexSigner(config: SignerConfig): SigningAccount {
	if (config.type === SignerType.PrivateKey) {
		const account = privateKeyToAccount(config.privateKey)
		const signRawHash = async (hash: HexString) => {
			const signature = await sign({
				hash,
				privateKey: config.privateKey,
			})
			const yParity =
				signature.yParity ??
				(signature.v !== undefined ? Number(signature.v >= 27n ? signature.v - 27n : signature.v) : undefined)
			if (yParity !== 0 && yParity !== 1) {
				throw new Error("Failed to derive yParity from private key signature")
			}
			return {
				r: signature.r as HexString,
				s: signature.s as HexString,
				yParity,
			}
		}
		return {
			mode: "privateKey",
			account,
			signBidMessage: (messageHash: HexString) => account.signMessage({ message: { raw: messageHash } }),
			signRawHash,
		}
	}

	if (config.type === SignerType.MpcVault) {
		const { account, service } = createMpcVaultAccount(config.mpcVault)
		const signRawHash = async (hash: HexString) => {
			const signature = await service.signRawHash(hash)
			return parseSignature(signature)
		}
		return {
			mode: "mpcVault",
			account,
			signBidMessage: (messageHash: HexString, chainId: number) =>
				service.signPersonalMessage(messageHash, chainId),
			signRawHash,
		}
	}

	throw new Error(`Unsupported signer mode: ${(config as { type?: string }).type ?? "unknown"}`)
}

export function initializeSignerFromToml(signerTomlConfig?: SignerConfig): SigningAccount | undefined {
	return signerTomlConfig ? createSimplexSigner(signerTomlConfig) : undefined
}
