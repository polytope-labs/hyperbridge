import type { HexString } from "@hyperbridge/sdk"
import { keccak256, serializeTransaction, type TransactionSerializable } from "viem"
import { hashAuthorization } from "viem/utils"
import { MpcVaultService } from "../mpcvault"
import type { AuthorizationRequest, MpcVaultSignerConfig, Signer, SignerTransaction } from "../types"

/**
 * Signs through MPCVault's signing ceremony.
 *
 * MPCVault takes structured requests, not viem shapes, so this implements the
 * interface against the service directly — no viem account underneath. It signs
 * transactions structurally, which is the point of MPC custody: the vault's
 * policy engine sees to/value/data rather than a digest.
 *
 * The exception is EIP-7702. MPCVault's structured send (`evmSendCustom`) has no
 * field for an authorization list, so a set-code transaction sent that way would
 * go out as a plain transfer with the delegation silently dropped. Those are
 * serialised and raw-signed here instead.
 */
export function mpcVaultSigner(config: MpcVaultSignerConfig): Signer {
	const service = new MpcVaultService({
		apiToken: config.apiToken,
		vaultUuid: config.vaultUuid,
		accountAddress: config.accountAddress,
		callbackClientSignerPublicKey: config.callbackClientSignerPublicKey,
		grpcTarget: config.grpcTarget,
	})

	return {
		address: config.accountAddress,
		mode: "mpcVault",
		// MPCVault has no structured EIP-7702 encoding, so the tuple is hashed here
		// and signed as a digest.
		signAuthorization: (auth: AuthorizationRequest) =>
			service.signRawHashComponents(
				hashAuthorization({
					address: auth.contractAddress,
					chainId: auth.chainId,
					nonce: auth.nonce,
				}) as HexString,
			),
		// The vault scopes a signing request to a chain, and EIP-712 already carries
		// one. Bigints are stringified because the payload is hashed server-side from
		// this JSON — a UserOperation's nonce and gas fields would otherwise throw.
		signTypedData: (typedData) =>
			service.signTypedData(
				JSON.stringify(typedData, (_k, v) => (typeof v === "bigint" ? v.toString() : v)),
				requireChainId(typedData.domain?.chainId),
			),
		signTransaction: async (tx: SignerTransaction) => {
			if (tx.authorizationList?.length) {
				const serializable = { ...tx, type: "eip7702" } as TransactionSerializable
				const signature = await service.signRawHashComponents(
					keccak256(serializeTransaction(serializable)) as HexString,
				)
				return serializeTransaction(serializable, signature) as HexString
			}
			return service.signTransaction({
				chainId: tx.chainId,
				to: tx.to,
				value: tx.value,
				data: tx.data,
				nonce: tx.nonce,
				gasLimit: tx.gas,
				maxFeePerGas: tx.maxFeePerGas,
				maxPriorityFeePerGas: tx.maxPriorityFeePerGas,
			})
		},
	}
}

function requireChainId(value: number | bigint | undefined): number {
	if (typeof value === "number" && Number.isFinite(value)) return value
	if (typeof value === "bigint") return Number(value)
	throw new Error("MPCVault needs a chain id: the typed-data payload has no domain.chainId")
}
