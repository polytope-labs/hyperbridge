import { EvmChain } from "@/chains/evm"
import { TronChain } from "@/chains/tron"
import { SubstrateChain } from "@/chains/substrate"
import type { PublicClient, TransactionReceipt } from "viem"
import type {
	GetResponseStorageValues,
	HexString,
	IEvmConfig,
	IGetRequest,
	IMessage,
	IPostRequest,
	ISubstrateConfig,
	StateMachineHeight,
	StateMachineIdParams,
} from "@/types"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import { isEvmChain, isSubstrateChain } from "@/utils"
import { ExpectedError } from "./utils/exceptions"
import { tronChainIds } from "./configs/chain"

export * from "@/chains/evm"
export * from "@/chains/substrate"
export * from "@/chains/intentsCoprocessor"
export * from "@/chains/tron"

/**
 * Type representing an ISMP message.
 */
export type IIsmpMessage = IRequestMessage | ITimeoutPostRequestMessage | IGetResponseMessage | IGetRequestMessage

export interface IRequestMessage {
	/**
	 * The kind of message.
	 */
	kind: "PostRequest"
	/**
	 * The requests to be posted.
	 */
	requests: IPostRequest[]
	/**
	 * The proof of the requests.
	 */
	proof: IProof
	/**
	 * The signer of the message.
	 */
	signer: HexString
}

export interface IGetRequestMessage {
	/**
	 * The kind of message.
	 */
	kind: "GetRequest"
	/**
	 * The requests to be posted.
	 */
	requests: IGetRequest[]
	/**
	 * The proof of the requests from the source chain.
	 */
	source: IProof
	/**
	 * The proof of the response from the target chain
	 */
	response: IProof
	/**
	 * The signer of the message.
	 */
	signer: HexString
}

export interface IGetResponse {
	/**
	 * The request that triggered this response.
	 */
	get: IGetRequest
	/**
	 * The response message.
	 */
	values: GetResponseStorageValues[]
}

export interface IGetResponseMessage {
	/**
	 * The kind of message.
	 */
	kind: "GetResponse"
	/**
	 * The responses to be posted.
	 */
	responses: IGetResponse[]
	/**
	 * The proof of the responses.
	 */
	proof: IProof
	/**
	 * The signer of the message.
	 */
	signer: HexString
}

export interface ITimeoutPostRequestMessage {
	/**
	 * The kind of message.
	 */
	kind: "TimeoutPostRequest"

	/**
	 * The requests to be posted.
	 */
	requests: IPostRequest[]

	/**
	 * The proof of the requests.
	 */
	proof: IProof
}

export interface IProof {
	/**
	 * The height of the proof.
	 */
	height: bigint
	/**
	 * The state machine identifier of the proof.
	 */
	stateMachine: string

	/**
	 * The associated consensus state identifier of the proof.
	 */
	consensusStateId: string

	/**
	 * The encoded storage proof
	 */
	proof: HexString
}

/**
 * Interface representing a chain.
 */
export interface IChain {
	/**
	 * Returns the configuration for this chain
	 */
	get config(): IEvmConfig | ISubstrateConfig

	/*
	 * Returns the current timestamp of the chain in seconds.
	 */
	timestamp(): Promise<bigint>

	/**
	 * Returns the state trie key for the request-receipt storage item for the given request commitment.
	 */
	requestReceiptKey(commitment: HexString): HexString

	/**
	 * Query and return the request-receipt for the given request commitment.
	 */
	queryRequestReceipt(commitment: HexString): Promise<HexString | undefined>

	/**
	 * Query and return the encoded storage proof for the provided keys at the given height.
	 * @param address - Optional contract address for EVM chains; defaults to host contract when omitted.
	 */
	queryStateProof(at: bigint, keys: HexString[], address?: HexString): Promise<HexString>

	/*
	 * Query and return the encoded storage proof for requests
	 */
	queryProof(message: IMessage, counterparty: string, at?: bigint): Promise<HexString>

	/*
	 * Encode an ISMP message into the appropriate calldata for this chain.
	 */
	encode(message: IIsmpMessage): HexString

	/**
	 * Get the latest state machine height for a given state machine ID.
	 */
	latestStateMachineHeight(stateMachineId: StateMachineIdParams): Promise<bigint>

	/**
	 * Get the challenge period for a given state machine ID.
	 */
	challengePeriod(stateMachineId: StateMachineIdParams): Promise<bigint>

	/**
	 * Get the update time for a statemachine height.
	 */
	stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint>
}

/**
 * Interface for EVM-compatible chains (EVM and Tron).
 * Extends IChain with methods required by IntentGatewayV2 and other EVM-specific protocols.
 */
export interface IEvmChain extends IChain {
	readonly configService: ChainConfigService
	readonly client: PublicClient
	getHostNonce(): Promise<bigint>
	quoteNative(request: IPostRequest | IGetRequest, fee: bigint): Promise<bigint>
	getFeeTokenWithDecimals(): Promise<{ address: HexString; decimals: number }>
	getPlaceOrderCalldata(txHash: string, intentGatewayAddress: string): Promise<HexString>
	broadcastTransaction(signedTransaction: any): Promise<TransactionReceipt>
}

/**
 * Returns the chain interface for a given state machine identifier.
 *
 * - For standard EVM chains, returns an `EvmChain`.
 * - For Substrate chains, returns a connected `SubstrateChain`.
 * - For Tron chains (identified by chain ID), constructs a `TronChain`
 *   that delegates EVM behavior to an internal `EvmChain` and manages
 *   its own TronWeb instance using the provided RPC URL.
 *
 * @param chainConfig - Chain configuration
 * @returns Chain interface
 */
export async function getChain(chainConfig: IEvmConfig | ISubstrateConfig): Promise<IChain> {
	if (isEvmChain(chainConfig.stateMachineId)) {
		return getEvmChain(chainConfig as IEvmConfig)
	}

	if (isSubstrateChain(chainConfig.stateMachineId)) {
		return getSubstrateChain(chainConfig as ISubstrateConfig)
	}

	throw new ExpectedError(`Unsupported chain: ${chainConfig.stateMachineId}`)
}

export function getEvmChain(chainConfig: IEvmConfig): IChain {
	const config = chainConfig as IEvmConfig
	const chainId = Number.parseInt(config.stateMachineId.split("-")[1])

	if (tronChainIds.has(chainId)) {
		return new TronChain({
			chainId,
			rpcUrl: config.rpcUrl,
			host: config.host,
			consensusStateId: config.consensusStateId,
		})
	}

	const evmChain = new EvmChain({
		chainId,
		rpcUrl: config.rpcUrl,
		host: config.host,
		consensusStateId: config.consensusStateId,
	})

	return evmChain
}

export async function getSubstrateChain(chainConfig: ISubstrateConfig) {
	const config = chainConfig as ISubstrateConfig
	const substrateChain = new SubstrateChain(config)

	await substrateChain.connect()

	return substrateChain
}
