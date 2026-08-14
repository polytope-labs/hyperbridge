import type { HexString } from "@hyperbridge/sdk"
import { concatHex, hashTypedData, keccak256, serializeTransaction, toHex, type TypedDataDefinition } from "viem"
import { createMpcVaultAccount } from "../mpcvault"
import type { MpcVaultSignerConfig, SigningAccount } from "../types"

export function createMpcVaultSigningAccount(config: MpcVaultSignerConfig): SigningAccount {
	const { account, service } = createMpcVaultAccount(config)
	const signRawHash = (hash: HexString) => service.signRawHashComponents(hash)
	return {
		mode: "mpcVault",
		account,
		signMessage: (messageHash: HexString, chainId: number) => service.signPersonalMessage(messageHash, chainId),
		signRawHash,
		// MPCVault's TYPE_SIGN_TYPED_DATA returns signatures that don't verify against
		// the payload's EIP-712 digest (#1132). Hash locally and sign the raw digest —
		// the same path the EIP-7702 flows use, whose signatures validate on-chain.
		signTypedData: async (typedData: unknown, _chainId?: number) => {
			const digest = hashTypedData(typedData as TypedDataDefinition)
			const { r, s, yParity } = await service.signRawHashComponents(digest as HexString)
			return concatHex([r, s, toHex(27 + yParity, { size: 1 })]) as HexString
		},
		sendEip7702DelegationTransaction: async (args) => {
			const chainNonce = await args.publicClient.getTransactionCount({
				address: args.authorityAddress,
				blockTag: "pending",
			})

			const txRequest = await args.walletClient.prepareTransactionRequest({
				to: args.authorityAddress,
				value: 0n,
				authorizationList: [args.authorization],
				chain: args.walletClient.chain,
				nonce: chainNonce,
			})

			const numericChainId = txRequest.chainId ?? args.chainIdFallback

			const txForSerialization = {
				...txRequest,
				type: "eip7702" as const,
				chainId: numericChainId,
				authorizationList: [args.authorization],
				nonce: chainNonce,
				gas: args.gasFloor,
			}

			const unsignedSerialized = serializeTransaction(txForSerialization)
			const txSigningHash = keccak256(unsignedSerialized)
			const signature = await signRawHash(txSigningHash as HexString)
			const signedSerialized = serializeTransaction(txForSerialization, signature)

			return (await args.publicClient.sendRawTransaction({
				serializedTransaction: signedSerialized,
			})) as HexString
		},
	}
}
