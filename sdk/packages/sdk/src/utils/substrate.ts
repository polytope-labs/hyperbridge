import { Struct, Vector, u8, u64, Tuple, Enum, _void, u32, Option } from "scale-ts"

export type IStateMachine =
	| {
			tag: "Evm"
			value: number
	  }
	| {
			tag: "Polkadot"
			value: number
	  }
	| {
			tag: "Kusama"
			value: number
	  }
	| {
			tag: "Substrate"
			value: number[]
	  }
	| {
			tag: "Tendermint"
			value: number[]
	  }

export const H256 = Vector(u8, 32)

export const EvmStateProof = Struct({
	/**
	 * Proof of the contract state.
	 */
	contractProof: Vector(Vector(u8)),
	/**
	 * Proof of the storage state.
	 */
	storageProof: Vector(Tuple(Vector(u8), Vector(Vector(u8)))),
})

export const SubstrateHashing = Enum({
	/* For chains that use keccak as their hashing algo */
	Keccak: _void,
	/* For chains that use blake2b as their hashing algo */
	Blake2: _void,
})

export const SubstrateStateMachineProof = Struct({
	/**
	 * The hasher used to hash the state machine state.
	 */
	hasher: SubstrateHashing,
	/**
	 * Proof of the state machine state.
	 */
	storageProof: Vector(Vector(u8)),
})

export const SubstrateStateProof = Enum({
	/*
	 * Uses overlay root for verification
	 */
	OverlayProof: SubstrateStateMachineProof,
	/*
	 * Uses state root for verification
	 */
	StateProof: SubstrateStateMachineProof,
})

export const BasicProof = Vector(Vector(u8))

export const LeafIndexAndPos = Struct({
	/*
	 * Leaf index
	 */
	leafIndex: u64,
	/*
	 * Leaf position in the MMR
	 */
	pos: u64,
})

export const MmrProof = Struct({
	/*
	 * Proof of the leaf index and position.
	 */
	leafIndexAndPos: Vector(LeafIndexAndPos),
	/*
	 * Proof of the leaf data.
	 */
	leafCount: u64,
	/*
	 * Proof elements (hashes of siblings of inner nodes on the path to the leaf).
	 */
	items: Vector(H256),
})

export const ConsensusStateId = Vector(u8, 4)

export const ConsensusMessage = Struct({
	/*
	 * Consensus message data.
	 */
	consensusProof: Vector(u8),
	/*
	 * Consensus state Id
	 */
	consensusStateId: ConsensusStateId,
	/*
	 * Public key of the sender
	 */
	signer: Vector(u8),
})

export const FraudProofMessage = Struct({
	/*
	 * The first valid consensus proof
	 */
	proof1: Vector(u8),
	/*
	 * The second valid consensus proof
	 */
	proof2: Vector(u8),
	/*
	 * Consensus state Id
	 */
	consensusStateId: ConsensusStateId,
})

export const StateMachine = Enum({
	/*
	 * Evm state machines
	 */
	Evm: u32,
	/*
	 * Polkadot parachains
	 */
	Polkadot: u32,
	/*
	 * Kusama parachains
	 */
	Kusama: u32,
	/*
	 * Substrate-based standalone chain
	 */
	Substrate: ConsensusStateId,
	/*
	 * Tendermint chains
	 */
	Tendermint: ConsensusStateId,
})

export const StateMachineId = Struct({
	/*
	 * The state machine id
	 */
	id: StateMachine,
	/*
	 * The consensus state id
	 */
	consensusStateId: ConsensusStateId,
})

export const StateMachineHeight = Struct({
	/*
	 * The state machine id
	 */
	id: StateMachineId,
	/*
	 * The height of the state machine
	 */
	height: u64,
})

export const Proof = Struct({
	/*
	 * The height of the state machine
	 */
	height: StateMachineHeight,
	/*
	 * The proof
	 */
	proof: Vector(u8),
})

export const PostRequest = Struct({
	/*
	 * The source state machine of this request.
	 */
	source: StateMachine,
	/*
	 * The destination state machine of this request.
	 */
	dest: StateMachine,
	/*
	 * The nonce of this request on the source chain
	 */
	nonce: u64,
	/*
	 * Module identifier of the sending module
	 */
	from: Vector(u8),
	/*
	 * Module identifier of the receiving module
	 */
	to: Vector(u8),
	/*
	 * Timestamp which this request expires in seconds.
	 */
	timeoutTimestamp: u64,
	/*
	 * Encoded request body
	 */
	body: Vector(u8),
})

export const PostResponse = Struct({
	/*
	 * The request that triggered this response.
	 */
	post: PostRequest,
	/*
	 * The response message.
	 */
	response: Vector(u8),
	/*
	 * Timestamp at which this response expires in seconds.
	 */
	timeoutTimestamp: u64,
})

export const GetRequest = Struct({
	/*
	 * The source state machine of this request.
	 */
	source: StateMachine,
	/*
	 * The destination state machine of this request.
	 */
	dest: StateMachine,
	/*
	 * The nonce of this request on the source chain
	 */
	nonce: u64,
	/*
	 * Module identifier of the sending module
	 */
	from: Vector(u8),
	/*
	 * Raw Storage keys that would be used to fetch the values from the counterparty
	 * For deriving storage keys for ink contract fields follow the guide in the link below
	 * `<https://use.ink/datastructures/storage-in-metadata#a-full-example>`
	 * Substrate Keys
	 */
	keys: Vector(Vector(u8)),
	/*
	 * The height of the state machine
	 */
	height: u64,
	/*
	 * Some application-specific metadata relating to this request
	 */
	context: Vector(u8),
	/*
	 * Host timestamp at which this request expires in seconds
	 */
	timeoutTimestamp: u64,
})

export const StorageValue = Struct({
	/*
	 * The request storage keys
	 */
	key: Vector(u8),
	/*
	 * The verified value
	 */
	value: Option(Vector(u8)),
})

export const GetResponse = Struct({
	/*
	 * The Get request that triggered this response.
	 */
	get: GetRequest,
	/*
	 * Values derived from the state proof
	 */
	values: Vector(StorageValue),
})

export const Request = Enum({
	/*
	 * A post request allows a module on a state machine to send arbitrary bytes to another module
	 * living in another state machine.
	 */
	Post: PostRequest,
	/*
	 * A get request allows a module on a state machine to read the storage of another module
	 * living in another state machine.
	 */
	Get: GetRequest,
})

export const Response = Enum({
	/*
	 * The response to a POST request
	 */
	Post: PostResponse,
	/*
	 * The response to a GET request
	 */
	Get: GetResponse,
})

export const RequestMessage = Struct({
	/*
	 * Requests from source chain
	 */
	requests: Vector(PostRequest),
	/*
	 * Membership batch proof for these requests
	 */
	proof: Proof,
	/*
	 * Signer information. Ideally should be their account identifier
	 */
	signer: Vector(u8),
})

export const RequestResponse = Enum({
	/*
	 * A set of requests
	 */
	Request: Vector(Request),
	/*
	 * A set of responses
	 */
	Response: Vector(Response),
})

export const ResponseMessage = Struct({
	/*
	 * A set of either POST requests or responses to be handled
	 */
	datagram: Vector(RequestResponse),
	/*
	 * Membership batch proof for these requests/responses
	 */
	proof: Proof,
	/*
	 * Signer information. Ideally should be their account identifier
	 */
	signer: Vector(u8),
})

export const TimeoutMessage = Enum({
	/*
	 * A non memership proof for POST requests
	 */
	Post: Struct({
		/*
		 * Timed out requests
		 */
		requests: Vector(Request),
		/*
		 * Membership batch proof for these requests
		 */
		proof: Proof,
	}),
	/*
	 * A non memership proof for POST responses
	 */
	PostResponse: Struct({
		/*
		 * Timed out responses
		 */
		responses: Vector(Response),
		/*
		 * Membership batch proof for these responses
		 */
		proof: Proof,
	}),
	/*
	 * There are no proofs for Get timeouts
	 */
	Get: Struct({
		/*
		 * Timed out requests
		 */
		requests: Vector(Request),
	}),
})

export const GetRequestsWithProof = Struct({
	/*
	 * Requests to be fetched
	 */
	requests: Vector(GetRequest),

	/*
	 * Membership batch proof for these requests
	 */
	source: Proof,

	/*
	 * Storage proof for these responses
	 */
	response: Proof,

	/*
	 * Signer information. Ideally should be their account identifier
	 */
	signer: Vector(u8),
})

export const Message = Enum({
	/*
	 * A consensus update message
	 */
	ConsensusMessage: ConsensusMessage,
	/*
	 * A fraud proof message
	 */
	FraudProofMessage: FraudProofMessage,
	/*
	 * A request message
	 */
	RequestMessage: RequestMessage,
	/*
	 * A response message
	 */
	ResponseMessage: ResponseMessage,
	/*
	 * A request timeout message
	 */
	TimeoutMessage: TimeoutMessage,
})
