import type { TransactionReceipt } from "viem"
import type { HexString } from "@/types"
import type { IProof } from "@/chain"
import type { RequestStatusWithMetadata } from "@/types"

/**
 * ABI-encoded function selector for the IntentGatewayV2 `placeOrder` function,
 * including the full tuple signature of the OrderV2 struct.
 */
export const PLACE_ORDER_SELECTOR =
	"placeOrder((bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes)),bytes32)"

/**
 * ABI tuple type string for the OrderV2 struct used when ABI-encoding or
 * decoding order data without a full ABI artifact.
 */
export const ORDER_V2_PARAM_TYPE =
	"(bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes))"

/**
 * Default graffiti value: a bytes32 zero word.
 *
 * When no graffiti is supplied to `placeOrder`, this value is used,
 * indicating no orderflow attribution or revenue-share tag.
 */
export const DEFAULT_GRAFFITI = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

/**
 * ERC-7821 execution mode selector for single-batch execution.
 *
 * The first byte `0x01` indicates batch mode; all remaining bytes are
 * reserved and set to zero.
 */
export const ERC7821_BATCH_MODE = "0x0100000000000000000000000000000000000000000000000000000000000000" as HexString

/**
 * Bundler RPC method names for ERC-4337 operations.
 *
 * Values map to JSON-RPC method strings sent to a 4337 bundler endpoint.
 * `PIMLICO_GET_USER_OPERATION_GAS_PRICE` is a Pimlico-specific extension.
 */
export const BundlerMethod = {
	/** Submits a UserOperation to the bundler mempool. */
	ETH_SEND_USER_OPERATION: "eth_sendUserOperation",
	/** Retrieves the receipt for a previously submitted UserOperation by its hash. */
	ETH_GET_USER_OPERATION_RECEIPT: "eth_getUserOperationReceipt",
	/** Estimates gas limits for a UserOperation before submission. */
	ETH_ESTIMATE_USER_OPERATION_GAS: "eth_estimateUserOperationGas",
	/** Pimlico-specific method to fetch recommended EIP-1559 gas prices for UserOperations. */
	PIMLICO_GET_USER_OPERATION_GAS_PRICE: "pimlico_getUserOperationGasPrice",
	/** Alchemy (Rundler) method to fetch recommended priority fee for UserOperations. */
	RUNDLER_MAX_PRIORITY_FEE_PER_GAS: "rundler_maxPriorityFeePerGas",
} as const

/** Union of all valid bundler RPC method name strings. */
export type BundlerMethod = (typeof BundlerMethod)[keyof typeof BundlerMethod]

/**
 * Response payload returned by a bundler for
 * `eth_estimateUserOperationGas`.
 *
 * All gas values are returned as hex strings.
 */
export interface BundlerGasEstimate {
	/** Gas required for pre-verification processing (hex). */
	preVerificationGas: HexString
	/** Gas limit for the account verification step (hex). */
	verificationGasLimit: HexString
	/** Gas limit for the main execution call (hex). */
	callGasLimit: HexString
	/** Gas limit for paymaster verification, if a paymaster is used (hex). */
	paymasterVerificationGasLimit?: HexString
	/** Gas limit for paymaster post-operation hook, if a paymaster is used (hex). */
	paymasterPostOpGasLimit?: HexString
}

/**
 * Response payload returned by Pimlico's
 * `pimlico_getUserOperationGasPrice` method.
 *
 * Provides EIP-1559 fee recommendations at three priority tiers.
 * Each field is optional; callers should fall back from `fast` → `standard` → `slow`.
 */
export interface PimlicoGasPriceEstimate {
	/** Low-priority fee recommendation. */
	slow: {
		/** Maximum total fee per gas (hex). */
		maxFeePerGas: HexString
		/** Maximum miner tip per gas (hex). */
		maxPriorityFeePerGas: HexString
	}
	/** Medium-priority fee recommendation. */
	standard: {
		/** Maximum total fee per gas (hex). */
		maxFeePerGas: HexString
		/** Maximum miner tip per gas (hex). */
		maxPriorityFeePerGas: HexString
	}
	/** High-priority fee recommendation for fastest inclusion. */
	fast: {
		/** Maximum total fee per gas (hex). */
		maxFeePerGas: HexString
		/** Maximum miner tip per gas (hex). */
		maxPriorityFeePerGas: HexString
	}
}

/**
 * Tagged union of all status updates that can be yielded by the cancel-order
 * async generator.
 *
 * The `status` field is the discriminant. Consumers should switch on it to
 * handle each stage of the cancellation lifecycle:
 *
 * - `DESTINATION_FINALIZED` – a state proof from the destination chain is ready.
 * - `AWAITING_CANCEL_TRANSACTION` – the caller must sign and submit the cancel tx.
 * - `CANCEL_STARTED` – the cancel transaction was confirmed on-chain.
 * - `SOURCE_FINALIZED` – the cancel request has been finalised on the source chain.
 * - `HYPERBRIDGE_DELIVERED` – the cancel message has been delivered to Hyperbridge.
 * - `HYPERBRIDGE_FINALIZED` – the cancel message has been finalised on Hyperbridge.
 * - `CANCELLATION_COMPLETE` – the escrow has been refunded; cancellation is done.
 */
export type CancelEvent =
	| { status: "DESTINATION_FINALIZED"; proof: IProof }
	| { status: "AWAITING_CANCEL_TRANSACTION"; data: HexString; to: HexString; value: bigint }
	| { status: "CANCEL_STARTED"; receipt: TransactionReceipt }
	| {
			status: "SOURCE_FINALIZED"
			metadata: Extract<RequestStatusWithMetadata, { status: "SOURCE_FINALIZED" }>["metadata"]
	  }
	| {
			status: "HYPERBRIDGE_DELIVERED"
			metadata: Extract<RequestStatusWithMetadata, { status: "HYPERBRIDGE_DELIVERED" }>["metadata"]
	  }
	| {
			status: "HYPERBRIDGE_FINALIZED"
			metadata: Extract<RequestStatusWithMetadata, { status: "HYPERBRIDGE_FINALIZED" }>["metadata"]
	  }
	| { status: "CANCELLATION_COMPLETE"; blockNumber: number; transactionHash: HexString }

import type { IEvmChain } from "@/chain"
import type { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { Swap } from "@/utils/swap"
import type { createSessionKeyStorage, createCancellationStorage, createUsedUserOpsStorage } from "@/storage"

/**
 * Shared runtime context passed to every IntentsV2 sub-module.
 *
 * All sub-modules (OrderPlacer, OrderExecutor, BidManager, etc.) receive a
 * reference to this object so they can share fee-token caches, storage
 * adapters, and chain clients without duplicating initialisation logic.
 */
export interface IntentGatewayContext {
	/** EVM chain on which orders are placed and escrowed. */
	source: IEvmChain
	/** EVM chain on which solvers fill orders and receive outputs. */
	dest: IEvmChain
	/** Hyperbridge coprocessor client used to fetch solver bids and submit UserOperations. */
	intentsCoprocessor?: IntentsCoprocessor
	/** URL of the ERC-4337 bundler endpoint for gas estimation and UserOp submission. */
	bundlerUrl?: string
	/**
	 * In-memory TTL cache keyed by state-machine ID.
	 * Stores fee-token address, decimals, and the timestamp of the last fetch.
	 */
	feeTokenCache: Map<string, { address: HexString; decimals: number; cachedAt: number }>
	/**
	 * In-memory cache of solver account contract bytecode, keyed by lowercased address.
	 * Used to inject solver code into state-overrides for gas estimation.
	 */
	solverCodeCache: Map<string, string>
	/** Persistent storage for ephemeral session keys generated per order. */
	sessionKeyStorage: ReturnType<typeof createSessionKeyStorage>
	/** Persistent storage for intermediate cancellation state (proofs, commitments). */
	cancellationStorage: ReturnType<typeof createCancellationStorage>
	/** Persistent storage for deduplication of already-submitted UserOperations. */
	usedUserOpsStorage: ReturnType<typeof createUsedUserOpsStorage>
	/** DEX-quote utility used for token pricing and gas-to-fee-token conversions. */
	swap: Swap
}
