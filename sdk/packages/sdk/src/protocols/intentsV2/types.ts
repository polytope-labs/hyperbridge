import type { HexString } from "@/types"
import type { IProof } from "@/chain"
import type { RequestStatusWithMetadata } from "@/types"

/** placeOrder function selector */
export const PLACE_ORDER_SELECTOR =
	"placeOrder((bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes)),bytes32)"

/** placeOrder function parameter type */
export const ORDER_V2_PARAM_TYPE =
	"(bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes))"

/** Default graffiti value (bytes32 zero) */
export const DEFAULT_GRAFFITI = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

/** ERC-7821 single batch execution mode */
export const ERC7821_BATCH_MODE = "0x0100000000000000000000000000000000000000000000000000000000000000" as HexString

/** Bundler RPC method names for ERC-4337 operations */
export const BundlerMethod = {
	ETH_SEND_USER_OPERATION: "eth_sendUserOperation",
	ETH_GET_USER_OPERATION_RECEIPT: "eth_getUserOperationReceipt",
	ETH_ESTIMATE_USER_OPERATION_GAS: "eth_estimateUserOperationGas",
} as const

export type BundlerMethod = (typeof BundlerMethod)[keyof typeof BundlerMethod]

/** Response from bundler's eth_estimateUserOperationGas */
export interface BundlerGasEstimate {
	preVerificationGas: HexString
	verificationGasLimit: HexString
	callGasLimit: HexString
	paymasterVerificationGasLimit?: HexString
	paymasterPostOpGasLimit?: HexString
}

/** Event map for cancellation status updates */
export interface CancelEventMap {
	DESTINATION_FINALIZED: { proof: IProof }
	AWAITING_GET_REQUEST: undefined
	AWAITING_CANCEL_TRANSACTION: { calldata: HexString; to: HexString }
	SOURCE_FINALIZED: { metadata: { blockNumber: number } }
	HYPERBRIDGE_DELIVERED: RequestStatusWithMetadata
	HYPERBRIDGE_FINALIZED: RequestStatusWithMetadata
	SOURCE_PROOF_RECEIVED: IProof
	CANCELLATION_COMPLETE: { metadata: { blockNumber: number } }
}

export type CancelEvent = {
	[K in keyof CancelEventMap]: { status: K; data: CancelEventMap[K] }
}[keyof CancelEventMap]

import type { IEvmChain } from "@/chain"
import type { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { Swap } from "@/utils/swap"
import type { createSessionKeyStorage, createCancellationStorage } from "@/storage"

/** Shared context for IntentsV2 sub-modules */
export interface IntentsV2Context {
	source: IEvmChain
	dest: IEvmChain
	intentsCoprocessor?: IntentsCoprocessor
	bundlerUrl?: string
	feeTokenCache: Map<string, { address: HexString; decimals: number; cachedAt: number }>
	solverCodeCache: Map<string, string>
	sessionKeyStorage: ReturnType<typeof createSessionKeyStorage>
	cancellationStorage: ReturnType<typeof createCancellationStorage>
	swap: Swap
}
