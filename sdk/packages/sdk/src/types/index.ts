import type { ConsolaInstance } from "consola"
import type { GraphQLClient } from "graphql-request"
import type { Hex, Log } from "viem"

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

export enum OrderStatus {
	PLACED = "PLACED",
	FILLED = "FILLED",
	REDEEMED = "REDEEMED",
	REFUNDED = "REFUNDED",
}

export enum TeleportStatus {
	TELEPORTED = "TELEPORTED",
	RECEIVED = "RECEIVED",
	REFUNDED = "REFUNDED",
}

export interface TokenGatewayAssetTeleportedResponse {
	tokenGatewayAssetTeleporteds: {
		nodes: Array<{
			id: string
			from: string
			to: string
			sourceChain: string
			destChain: string
			commitment: string
			amount: string
			usdValue: string
			assetId: string
			redeem: boolean
			status: TeleportStatus
			createdAt: string
			blockNumber: string
			blockTimestamp: string
			transactionHash: string
			statusMetadata: {
				nodes: Array<{
					status: TeleportStatus
					chain: string
					timestamp: string
					blockNumber: string
					blockHash: string
					transactionHash: string
				}>
			}
		}>
	}
}

export interface TokenGatewayAssetTeleportedWithStatus {
	id: string
	from: string
	to: string
	sourceChain: string
	destChain: string
	commitment: string
	amount: bigint
	usdValue: string
	assetId: string
	redeem: boolean
	status: TeleportStatus
	createdAt: Date
	blockNumber: bigint
	blockTimestamp: bigint
	transactionHash: string
	statuses: Array<{
		status: TeleportStatus
		metadata: {
			blockHash: string
			blockNumber: number
			transactionHash: string
			timestamp: bigint
		}
	}>
}

export interface BlockMetadata {
	blockHash: string
	blockNumber: number
	transactionHash: string
	calldata?: string
	timestamp?: number
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
	timestamp: number
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
				timestamp?: number
			}
	  }
	| {
			status: RequestStatus["SOURCE_FINALIZED"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
			}
	  }
	| {
			status: RequestStatus["HYPERBRIDGE_DELIVERED"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
			}
	  }
	| {
			status: RequestStatus["HYPERBRIDGE_FINALIZED"]
			metadata: {
				calldata: Hex
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
			}
	  }
	| {
			status: RequestStatus["DESTINATION"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
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
				timestamp?: number
			}
	  }
	| {
			status: TimeoutStatus["HYPERBRIDGE_TIMED_OUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
			}
	  }
	| {
			status: TimeoutStatus["HYPERBRIDGE_FINALIZED_TIMEOUT"]
			metadata: {
				calldata: Hex
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
			}
	  }
	| {
			status: TimeoutStatus["TIMED_OUT"]
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp?: number
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
		nodes: {
			height: number
			chain: string
			blockHash: string
			blockNumber: number
			transactionHash: string
			transactionIndex: number
			stateMachineId: string
			createdAt: string
		}[]
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
	stateId: { Evm?: number; Substrate?: HexString; Polkadot?: number; Kusama?: number }
	consensusStateId: HexString
}

/**
 * Configuration for a blockchain chain
 */
export interface ChainConfig {
	/**
	 * The unique identifier for the chain
	 */
	chainId: number

	/**
	 * The RPC URL to connect to the chain
	 */
	rpcUrl: string

	/**
	 * The address of the IntentGateway contract on this chain
	 */
	intentGatewayAddress: string
}

/**
 * Represents token information for an order
 */
export interface TokenInfo {
	/**
	 * The address of the ERC20 token
	 * address(0) is used as a sentinel for the native token
	 */
	token: HexString

	/**
	 * The amount of the token
	 */
	amount: bigint
}

/**
 * Represents payment information for an order
 */
export interface PaymentInfo extends TokenInfo {
	/**
	 * The address to receive the output tokens
	 */
	beneficiary: HexString
}

/**
 * Represents an order in the IntentGateway
 */
export interface Order {
	/**
	 * The unique identifier for the order
	 */
	id?: string

	/**
	 * The address of the user who is initiating the transfer
	 */
	user: HexString

	/**
	 * The state machine identifier of the origin chain
	 */
	sourceChain: string

	/**
	 * The state machine identifier of the destination chain
	 */
	destChain: string

	/**
	 * The block number by which the order must be filled on the destination chain
	 */
	deadline: bigint

	/**
	 * The nonce of the order
	 */
	nonce: bigint

	/**
	 * Represents the dispatch fees associated with the IntentGateway
	 */
	fees: bigint

	/**
	 * The tokens that the filler will provide
	 */
	outputs: PaymentInfo[]

	/**
	 * The tokens that are escrowed for the filler
	 */
	inputs: TokenInfo[]

	/**
	 * A bytes array to store the calls if any
	 */
	callData: HexString

	// Additional Data
	/**
	 * The transaction hash of the order
	 */
	transactionHash?: HexString
}

export interface DecodedOrderPlacedLog extends Log {
	eventName: string
	args: {
		user: HexString
		sourceChain: Hex
		destChain: Hex
		deadline: bigint
		nonce: bigint
		fees: bigint
		outputs: Array<{
			token: HexString
			amount: bigint
			beneficiary: HexString
		}>
		inputs: Array<{
			token: HexString
			amount: bigint
		}>
		callData: HexString
	}
	transactionHash: HexString
}

/**
 * Options for filling an order
 */
export interface FillOptions {
	/**
	 * The fee paid to the relayer for processing transactions
	 */
	relayerFee: bigint
}

/**
 * Options for canceling an order
 */
export interface CancelOptions {
	/**
	 * The fee paid to the relayer for processing transactions
	 */
	relayerFee: string

	/**
	 * Stores the height value
	 */
	height: string
}

/**
 * Represents a new deployment of IntentGateway
 */
export interface NewDeployment {
	/**
	 * Identifier for the state machine
	 */
	stateMachineId: HexString

	/**
	 * The gateway identifier
	 */
	gateway: HexString
}

/**
 * Represents the body of a request
 */
export interface RequestBody {
	/**
	 * Represents the commitment of an order
	 */
	commitment: HexString

	/**
	 * Stores the identifier for the beneficiary
	 */
	beneficiary: HexString

	/**
	 * An array of token identifiers
	 */
	tokens: TokenInfo[]
}

/**
 * Represents the parameters for the IntentGateway module
 */
export interface IntentGatewayParams {
	/**
	 * The address of the host contract
	 */
	host: string

	/**
	 * Address of the dispatcher contract responsible for handling intents
	 */
	dispatcher: string
}

/**
 * Enum representing the different kinds of incoming requests
 */
export enum RequestKind {
	/**
	 * Identifies a request for redeeming an escrow
	 */
	RedeemEscrow = 0,

	/**
	 * Identifies a request for recording new contract deployments
	 */
	NewDeployment = 1,

	/**
	 * Identifies a request for updating parameters
	 */
	UpdateParams = 2,
}

/**
 * Configuration for the IntentFiller
 */
export interface FillerConfig {
	/**
	 * Policy for determining confirmation requirements
	 */
	confirmationPolicy: {
		getConfirmationBlocks: (chainId: number, amount: bigint) => number
	}

	/**
	 * Maximum number of orders to process concurrently
	 */
	maxConcurrentOrders?: number

	/**
	 * Minimum profitability threshold to consider filling an order
	 * Expressed as a percentage (e.g., 0.5 = 0.5%)
	 */
	minProfitabilityThreshold?: number

	/**
	 * Gas price strategy for each chain
	 * Maps chainId to a gas price strategy function
	 */
	gasPriceStrategy?: Record<string, () => Promise<string>>

	/**
	 * Maximum gas price willing to pay for each chain
	 * Maps chainId to maximum gas price in wei
	 */
	maxGasPrice?: Record<string, string>

	/**
	 * Retry configuration for failed transactions
	 */
	retryConfig?: {
		/**
		 * Maximum number of retry attempts
		 */
		maxAttempts: number

		/**
		 * Initial delay between retries in ms
		 */
		initialDelayMs: number
	}

	/**
	 * Configuration for the pending queue
	 */
	pendingQueueConfig?: {
		/**
		 * Delay in milliseconds before rechecking an order for confirmations
		 * Default: 30000 (30 seconds)
		 */
		recheckDelayMs?: number

		/**
		 * Maximum number of times to recheck an order before giving up
		 * Default: 10
		 */
		maxRechecks?: number
	}
}

/**
 * Result of an order execution attempt
 */
export interface ExecutionResult {
	/**
	 * Whether the execution was successful
	 */
	success: boolean

	/**
	 * The transaction hash if successful
	 */
	txHash?: string

	/**
	 * Error message if unsuccessful
	 */
	error?: string

	/**
	 * Gas used by the transaction
	 */
	gasUsed?: string

	/**
	 * Gas price used for the transaction
	 */
	gasPrice?: string

	/**
	 * Total transaction cost in wei
	 */
	txCost?: string

	/**
	 * Block number when the transaction was confirmed
	 */
	confirmedAtBlock?: number

	/**
	 * Timestamp when the transaction was confirmed
	 */
	confirmedAt?: Date

	/**
	 * Actual profitability achieved
	 */
	actualProfitability?: number

	/**
	 * Strategy used to fill the order
	 */
	strategyUsed?: string

	/**
	 * Any tokens exchanged during the fill process
	 */
	exchanges?: Array<{
		fromToken: HexString
		toToken: HexString
		fromAmount: string
		toAmount: string
		exchangeRate: string
	}>

	/**
	 * The time it took to fill the order
	 */
	processingTimeMs?: number
}

/**
 * Represents a dispatch post for cross-chain communication
 */
export interface DispatchPost {
	/**
	 * Bytes representation of the destination state machine
	 */
	dest: HexString

	/**
	 * The destination module
	 */
	to: HexString

	/**
	 * The request body
	 */
	body: HexString

	/**
	 * Timeout for this request in seconds
	 */
	timeout: bigint

	/**
	 * The amount put up to be paid to the relayer,
	 * this is charged in `IIsmpHost.feeToken` to `msg.sender`
	 */
	fee: bigint

	/**
	 * Who pays for this request?
	 */
	payer: HexString
}

export interface DispatchGet {
	/**
	 * Bytes representation of the destination state machine
	 */
	dest: HexString

	/**
	 * Height at which to read the state machine
	 */
	height: bigint

	/**
	 * Raw storage keys to fetch values from the counterparty
	 */
	keys: HexString[]

	/**
	 * Timeout for this request in seconds
	 */
	timeout: bigint

	/**
	 * The amount put up to be paid to the relayer
	 */
	fee: bigint

	/**
	 * Context for the request
	 */
	context: HexString
}

export interface StateMachineHeight {
	id: {
		stateId: { Evm?: number; Substrate?: HexString; Polkadot?: number; Kusama?: number }
		consensusStateId: HexString
	}
	height: bigint
}

/**
 * The EvmHost protocol parameters
 */
export interface HostParams {
	/**
	 * The default timeout in seconds for messages. If messages are dispatched
	 * with a timeout value lower than this this value will be used instead
	 */
	defaultTimeout: bigint
	/**
	 * The default per byte fee
	 */
	perByteFee: bigint
	/**
	 * The cost for applications to access the hyperbridge state commitment.
	 * They might do so because the hyperbridge state contains the verified state commitments
	 * for all chains and they want to directly read the state of these chains state bypassing
	 * the ISMP protocol entirely.
	 */
	stateCommitmentFee: bigint
	/**
	 * The fee token contract address. This will typically be DAI.
	 * but we allow it to be configurable to prevent future regrets.
	 */
	feeToken: HexString
	/**
	 * The admin account, this only has the rights to freeze, or unfreeze the bridge
	 */
	admin: HexString
	/**
	 * Ismp message handler contract. This performs all verification logic
	 * needed to validate cross-chain messages before they are dispatched to local modules
	 */
	handler: HexString
	/**
	 * The authorized host manager contract, is itself an `IIsmpModule`
	 * which receives governance requests from the Hyperbridge chain to either
	 * withdraw revenue from the host or update its protocol parameters
	 */
	hostManager: HexString
	/**
	 * The local UniswapV2Router02 contract, used for swapping the native token to the feeToken.
	 */
	uniswapV2: HexString
	/**
	 * The unstaking period of Polkadot's validators. In order to prevent long-range attacks
	 */
	unStakingPeriod: bigint
	/**
	 * Minimum challenge period for state commitments in seconds
	 */
	challengePeriod: bigint
	/**
	 * The consensus client contract which handles consensus proof verification
	 */
	consensusClient: HexString
	/**
	 * State machines whose state commitments are accepted
	 */
	readonly stateMachines: readonly bigint[]
	/**
	 * The state machine identifier for hyperbridge
	 */
	hyperbridge: HexString
}

export enum EvmLanguage {
	Solidity,
	Vyper,
}

export interface OrderStatusMetadata {
	status: OrderStatus
	chain: string
	timestamp: bigint
	blockNumber: string
	transactionHash: string
	filler?: string
}

export interface OrderWithStatus {
	id: string
	user: string
	sourceChain: string
	destChain: string
	commitment: string
	deadline: bigint
	nonce: bigint
	fees: bigint
	inputTokens: string[]
	inputAmounts: bigint[]
	inputValuesUSD: string[]
	inputUSD: string
	outputTokens: string[]
	outputAmounts: bigint[]
	outputBeneficiaries: string[]
	calldata: string
	status: OrderStatus
	createdAt: Date
	blockNumber: bigint
	blockTimestamp: bigint
	transactionHash: string
	statuses: Array<{
		status: OrderStatus
		metadata: {
			blockHash: string
			blockNumber: number
			transactionHash: string
			timestamp: bigint
			filler?: string
		}
	}>
}

export interface OrderResponse {
	orderPlaceds: {
		nodes: Array<{
			id: string
			user: string
			sourceChain: string
			destChain: string
			commitment: string
			deadline: string
			nonce: string
			fees: string
			inputTokens: string[]
			inputAmounts: string[]
			inputValuesUSD: string[]
			inputUSD: string
			outputTokens: string[]
			outputAmounts: string[]
			outputBeneficiaries: string[]
			calldata: string
			status: OrderStatus
			createdAt: string
			blockNumber: string
			blockTimestamp: string
			transactionHash: string
			statusMetadata: {
				nodes: Array<{
					status: OrderStatus
					chain: string
					timestamp: string
					blockNumber: string
					blockHash: string
					transactionHash: string
					filler?: string
				}>
			}
		}>
	}
}
