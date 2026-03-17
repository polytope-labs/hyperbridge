import type { HexString } from "@hyperbridge/sdk"
import { toAccount, privateKeyToAccount, type Account } from "viem/accounts"
import { type FillerConfig, type MpcVaultConfig } from "../FillerConfigService"
import { MpcVaultService } from "./mpcvault"
import type { SimplexSigner } from "./types"

function requireChainId(value: unknown, context: string): number {
	if (typeof value === "number" && Number.isFinite(value)) return value
	if (typeof value === "bigint") return Number(value)
	throw new Error(`Missing chainId for MPCVault ${context}`)
}

function createMpcVaultAccount(config: MpcVaultConfig): { account: Account; service: MpcVaultService } {
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

export function createSimplexSigner(config: FillerConfig): SimplexSigner {
	if (config.privateKey) {
		const account = privateKeyToAccount(config.privateKey as HexString)
		return {
			mode: "privateKey",
			account,
			signBidMessage: (messageHash: HexString) => account.signMessage({ message: { raw: messageHash } }),
		}
	}

	if (config.mpcVault) {
		const { account, service } = createMpcVaultAccount(config.mpcVault)
		return {
			mode: "mpcVault",
			account,
			signBidMessage: (messageHash: HexString, chainId: number) =>
				service.signPersonalMessage(messageHash, chainId),
		}
	}

	throw new Error("Either simplex.privateKey or simplex.mpcVault must be configured")
}
