import { Hex } from "viem"

/**
 * Read-only storage query description for a cross-chain GET request.
 */
export interface GetRequest {
	// The source state machine of this request.
	source: string
	// The destination state machine of this request.
	dest: string
	// Module Id of the sending module
	from: Hex
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
	keys: Hex[]
	// Timestamp which this request expires in seconds.
	timeoutTimestamp: bigint
	context: Hex
}

/**
 * A single key/value pair returned in a GET response.
 */
export interface GetResponseStorageValues {
	key: Hex
	value: Hex
}

/**
 * Response payload to a prior {@link GetRequest}.
 */
export interface GetResponse {
	/**
	 * The request that triggered this response.
	 */
	get: GetRequest
	/**
	 * The response message.
	 */
	values: GetResponseStorageValues[]
}

/**
 * Executable POST request to a destination module.
 */
export interface PostRequest {
	// The source state machine of this request.
	source: string
	// The destination state machine of this request.
	dest: string
	// Module Id of the sending module
	from: Hex
	// Module ID of the receiving module
	to: Hex
	// The nonce of this request on the source chain
	nonce: bigint
	// Encoded request body.
	body: Hex
	// Timestamp which this request expires in seconds.
	timeoutTimestamp: bigint
}

/**
 * Response payload to a prior {@link PostRequest}.
 */
export interface PostResponse {
	// The request that triggered this response.
	post: PostRequest
	// The response message.
	response: string
	// Timestamp at which this response expires in seconds.
	timeoutTimestamp: bigint
}

/**
 * Batched GET request timeouts with proof for verification at a given height.
 */
export interface GetTimeoutMessage {
	timeouts: GetRequest[]
	height: StateMachineHeight
	proof: Hex[]
}

/**
 * Batched POST request timeouts with proof for verification at a given height.
 */
export interface PostRequestTimeoutMessage {
	timeouts: PostRequest[]
	height: StateMachineHeight
	proof: Hex[]
}

/**
 * Batched POST response timeouts with proof for verification at a given height.
 */
export interface PostResponseTimeoutMessage {
	timeouts: PostResponse[]
	height: StateMachineHeight
	proof: Hex[]
}

/**
 * Chain-unique state machine identifier and block height.
 */
export interface StateMachineHeight {
	stateMachineId: bigint
	height: bigint
}

/**
 * Batch of GET responses accompanied by inclusion proof.
 */
export interface GetResponseMessage {
	proof: Proof
	responses: {
		response: GetResponse
		index: bigint
		kIndex: bigint
	}[]
}

/**
 * Batch of POST requests accompanied by inclusion proof.
 */
export interface PostRequestMessage {
	proof: Proof
	requests: {
		request: PostRequest
		index: bigint
		kIndex: bigint
	}[]
}

/**
 * Batch of POST responses accompanied by inclusion proof.
 */
export interface PostResponseMessage {
	proof: Proof
	responses: {
		response: PostResponse
		index: bigint
		kIndex: bigint
	}[]
}

/**
 * Merkle multiproof and metadata to verify inclusion at a specific height.
 */
export interface Proof {
	height: StateMachineHeight
	multiproof: Hex[]
	leafCount: bigint
}
