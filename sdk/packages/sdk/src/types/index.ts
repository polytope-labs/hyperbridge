import type { ConsolaInstance } from "consola"
import type { GraphQLClient } from "graphql-request"
import type { Hex } from "viem"

export type HexString = `0x${string}`

export interface IConfig {
	// confuration object for the source chain
	source: IEvmConfig | ISubstrateConfig
	// confuration object for the destination chain
	dest: IEvmConfig | ISubstrateConfig
	// confuration object for hyperbridge
	hyperbridge: IHyperbridgeConfig
	// Flag to enable tracing console logs
	tracing?: boolean
}

export interface IEvmConfig {
	// rpc url of the chain
	rpcUrl: string
	// state machine identifier as a string
	stateMachineId: string
	// contract address of the `IsmpHost` on this chain
	host: string
	// consensus state identifier of this chain on hyperbridge
	consensusStateId: string
}

export interface ISubstrateConfig {
	// rpc url of the chain
	wsUrl: string
	// consensus state identifier of this chain on hyperbridge
	consensusStateId: string
	// consensus state identifier of this chain on hyperbridge
	hasher: "Keccak" | "Blake2"
	// state machine identifier as a string
	stateMachineId: string
}

export interface IHyperbridgeConfig {
	// websocket rpc endpoint for hyperbridge
	wsUrl: string
	// state machine identifier as a string
	stateMachineId: string
	// consensus state identifier of hyperbridge on the destination chain
	consensusStateId: string
}

export interface IPostRequest {
	// The source state machine of this request.
	source: string
	// The destination state machine of this request.
	dest: string
	// Module Id of the sending module
	from: HexString
	// Module ID of the receiving module
	to: HexString
	// The nonce of this request on the source chain
	nonce: bigint
	// Encoded request body.
	body: HexString
	// Timestamp which this request expires in seconds.
	timeoutTimestamp: bigint
}

export interface IGetRequest {
	// The source state machine of this request.
	source: string
	// The destination state machine of this request.
	dest: string
	// Module Id of the sending module
	from: HexString
	// The nonce of this request on the source chain
	nonce: bigint
	// Height at which to read the state machine.
	height: bigint
	/// Raw Storage keys that would be used to fetch the values from the counterparty
	/// For deriving storage keys for ink contract fields follow the guide in the link below
	/// `<https://use.ink/datastructures/storage-in-metadata#a-full-example>`
	/// The algorithms for calculating raw storage keys for different substrate pallet storage
	/// types are described in the following links
	/// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/map.rs#L34-L42>`
	/// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/double_map.rs#L34-L44>`
	/// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/nmap.rs#L39-L48>`
	/// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/value.rs#L37>`
	/// For fetching keys from EVM contracts each key should be 52 bytes
	/// This should be a concatenation of contract address and slot hash
	keys: HexString[]
	// Timestamp which this request expires in seconds.
	timeoutTimestamp: bigint
	context: HexString
}

export interface GetResponseStorageValues {
	key: HexString
	value: HexString
}

export interface IPostResponse {
	// The request that triggered this response.
	post: IPostRequest
	// The response message.
	response: string
	// Timestamp at which this response expires in seconds.
	timeoutTimestamp: bigint
}

export type IMessage = { Requests: HexString[] } | { Responses: HexString[] }

export type IndexerQueryClient = GraphQLClient

export interface ClientConfig {
	pollInterval: number
	queryClient: IndexerQueryClient
	tracing?: boolean
	source: IEvmConfig | ISubstrateConfig
	dest: IEvmConfig | ISubstrateConfig
	hyperbridge: IHyperbridgeConfig
}

export interface RetryConfig {
	maxRetries: number
	backoffMs: number
	logMessage?: string
	logger?: ConsolaInstance
}

export interface IsmpRequest {
	source: string
	dest: string
	from: string
	to: string
	nonce: bigint
	body: string
	timeoutTimestamp: bigint
	storage_key?: string
}

export const RequestStatus = Object.freeze({
	SOURCE: "SOURCE",
	SOURCE_FINALIZED: "SOURCE_FINALIZED",
	HYPERBRIDGE_DELIVERED: "HYPERBRIDGE_DELIVERED",
	HYPERBRIDGE_FINALIZED: "HYPERBRIDGE_FINALIZED",
	DESTINATION: "DESTINATION",
	TIMED_OUT: "TIMED_OUT",
	HYPERBRIDGE_TIMED_OUT: "HYPERBRIDGE_TIMED_OUT",
})
export type RequestStatus = typeof RequestStatus
export type RequestStatusKey = keyof typeof RequestStatus

export const TimeoutStatus = Object.freeze({
	PENDING_TIMEOUT: "PENDING_TIMEOUT",
	DESTINATION_FINALIZED_TIMEOUT: "DESTINATION_FINALIZED_TIMEOUT",
	HYPERBRIDGE_TIMED_OUT: "HYPERBRIDGE_TIMED_OUT",
	HYPERBRIDGE_FINALIZED_TIMEOUT: "HYPERBRIDGE_FINALIZED_TIMEOUT",
	TIMED_OUT: "TIMED_OUT",
})

export type TimeoutStatus = typeof TimeoutStatus
export type TimeoutStatusKey = keyof typeof TimeoutStatus

export type AllStatusKey = RequestStatusKey | TimeoutStatusKey

export enum HyperClientStatus {
	PENDING = "PENDING",
	SOURCE_FINALIZED = "SOURCE_FINALIZED",
	HYPERBRIDGE_FINALIZED = "HYPERBRIDGE_FINALIZED",
	HYPERBRIDGE_VERIFIED = "HYPERBRIDGE_VERIFIED",
	DESTINATION = "DESTINATION",
	TIMED_OUT = "TIMED_OUT",
	HYPERBRIDGE_TIMED_OUT = "HYPERBRIDGE_TIMED_OUT",
	ERROR = "ERROR",
}

export interface BlockMetadata {
	blockHash: string
	blockNumber: number
	transactionHash: string
	calldata?: string
}

export interface PostRequestStatus {
	status: RequestStatusKey
	metadata: Partial<BlockMetadata>
}

export interface PostRequestTimeoutStatus {
	status: TimeoutStatusKey
	metadata?: Partial<BlockMetadata>
}

export interface StateMachineUpdate {
	height: number
	chain: string
	blockHash: string
	blockNumber: number
	transactionHash: string
	transactionIndex: number
	stateMachineId: string
	createdAt: string
}

export interface RequestResponse {
	requests: {
		nodes: Array<{
			source: string
			dest: string
			to: HexString
			from: HexString
			nonce: bigint
			body: HexString
			timeoutTimestamp: bigint
			statusMetadata: {
				nodes: Array<{
					blockHash: string
					blockNumber: string
					timestamp: string
					chain: string
					status: string
					transactionHash: string
				}>
			}
		}>
	}
}

export interface GetRequestResponse {
	getRequests: {
		nodes: Array<{
			source: string
			dest: string
			to: HexString
			from: HexString
			nonce: bigint
			height: bigint
			keys: HexString[]
			context: HexString
			timeoutTimestamp: bigint
			statusMetadata: {
				nodes: Array<{
					blockHash: string
					blockNumber: string
					timestamp: string
					chain: string
					status: string
					transactionHash: string
				}>
			}
		}>
	}
}

export type RequestStatusWithMetadata =
	| {
			status: RequestStatus["SOURCE"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: RequestStatus["SOURCE_FINALIZED"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: RequestStatus["HYPERBRIDGE_DELIVERED"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: RequestStatus["HYPERBRIDGE_FINALIZED"]
			metadata: {
				calldata: Hex
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: RequestStatus["DESTINATION"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: TimeoutStatus["PENDING_TIMEOUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: TimeoutStatus["DESTINATION_FINALIZED_TIMEOUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: TimeoutStatus["HYPERBRIDGE_TIMED_OUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: TimeoutStatus["HYPERBRIDGE_FINALIZED_TIMEOUT"]
			metadata: {
				calldata: Hex
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }
	| {
			status: TimeoutStatus["TIMED_OUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
			}
	  }

interface GenericRequestWithStatuses {
	source: string
	dest: string
	from: HexString
	nonce: bigint
	timeoutTimestamp: bigint
	statuses: Array<RequestStatusWithMetadata>
}

export interface PostRequestWithStatus extends GenericRequestWithStatuses {
	to: HexString
	body: HexString
}

export interface GetRequestWithStatus extends GenericRequestWithStatuses {
	height: bigint
	keys: HexString[]
	context: HexString
}

export interface GetResponseByRequestIdResponse {
	getResponses: {
		nodes: Array<{
			id: string
			commitment: string
			responseMessage: string[]
		}>
	}
}

export interface ResponseCommitmentWithValues {
	commitment: string
	values: string[]
}

export interface RequestCommitment {
	requests: {
		nodes: Array<{
			id: string
			commitment: string
		}>
	}
}

export interface StateMachineResponse {
	stateMachineUpdateEvents: {
		nodes: StateMachineUpdate[]
	}
}

export interface AssetTeleported {
	id: string
	from: string
	to: string
	amount: bigint
	dest: string
	commitment: string
	createdAt: Date
	blockNumber: number
}

export interface AssetTeleportedResponse {
	assetTeleporteds: {
		nodes: AssetTeleported[]
	}
}

export interface StateMachineIdParams {
	stateId: { Evm: number }
	consensusStateId: HexString
}
