import { type ConsolaInstance, createConsola, LogLevels } from "consola"
import { maxBy } from "lodash-es"
import { pad, toHex } from "viem"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import type {
	AssetTeleported,
	AssetTeleportedResponse,
	BlockMetadata,
	ClientConfig,
	GetRequestResponse,
	GetRequestWithStatus,
	GetResponseByRequestIdResponse,
	HexString,
	OrderWithStatus,
	PostRequestStatus,
	PostRequestTimeoutStatus,
	PostRequestWithStatus,
	RequestStatusKey,
	RequestStatusWithMetadata,
	ResponseCommitmentWithValues,
	RetryConfig,
	StateMachineResponse,
	StateMachineUpdate,
	TokenGatewayAssetTeleportedWithStatus,
	TimeoutStatusKey,
} from "@/types"

import {
	STATE_MACHINE_UPDATES_BY_HEIGHT,
	STATE_MACHINE_UPDATES_BY_TIMESTAMP,
	ASSET_TELEPORTED_BY_PARAMS,
	GET_RESPONSE_BY_REQUEST_ID,
} from "@/queries"
import {
	COMBINED_STATUS_WEIGHTS,
	DEFAULT_POLL_INTERVAL,
	REQUEST_STATUS_WEIGHTS,
	TIMEOUT_STATUS_WEIGHTS,
	dateStringtoTimestamp,
	parseStateMachineId,
	postRequestCommitment,
	retryPromise,
	sleep,
	waitForChallengePeriod,
} from "@/utils"
import { getChain, type IChain, type SubstrateChain } from "@/chain"
import {
	_queryGetRequestInternal,
	_queryRequestInternal,
	_queryOrderInternal,
	_queryTokenGatewayAssetTeleportedInternal,
} from "./query-client"

import { OrderStatus, RequestStatus, TeleportStatus, TimeoutStatus } from "@/types"

import type { IndexerQueryClient } from "@/types"

/**
 * IndexerClient provides methods for interacting with the Hyperbridge indexer.
 *
 * This client facilitates querying and tracking cross-chain requests and their status
 * through the Hyperbridge protocol. It supports:
 *
 * - Querying state machine updates by block height or timestamp
 * - Retrieving request status information by transaction hash
 * - Monitoring request status changes through streaming interfaces
 * - Handling request timeout flows and related proof generation
 * - Tracking request finalization across source and destination chains
 *
 * The client implements automatic retries with exponential backoff for network
 * resilience and provides both simple query methods and advanced streaming
 * interfaces for real-time status tracking.
 *
 * The URLs provided in the configuration must point to archive nodes to allow the client to query for storage proofs
 * of potentially much older blocks. Regular nodes only store the state for recent blocks and will not be able
 * to provide the necessary proofs for cross-chain verification, especially in timeout scenarios.
 *
 * @example
 * ```typescript
 * const client = new IndexerClient({
 *   url: "https://indexer.hyperbridge.xyz/graphql",
 *   pollInterval: 2000,
 *   source: {
 *		stateMachineId: "EVM-1",
 * 		consensusStateId: "ETH0"
 *		rpcUrl: "",
 *		host: "0x87ea45..",
 * 	},
 *   dest: {
 *		stateMachineId: "EVM-42161",
 * 		consensusStateId: "ETH0"
 *		rpcUrl: "",
 *		host: "0x87ea42345..",
 * 	},
 *   hyperbridge: {
 *     stateMachineId: "POLKADOT-3367",
 *     consensusStateId: "DOT0"
 *     wsUrl: "ws://localhost:9944"
 *   }
 * });
 *
 * // Query a request status
 * const status = await client.queryRequestWithStatus("0x1234...");
 *
 * // Stream status updates
 * for await (const update of client.postRequestStatusStream("0x1234...")) {
 *   console.log(`Request status: ${update.status}`);
 * }
 * ```
 */
export class IndexerClient {
	/**
	 * GraphQL client used for making requests to the indexer
	 */
	private client: IndexerQueryClient

	/**
	 * Configuration for the IndexerClient including URLs, poll intervals, and chain-specific settings
	 */
	private config: ClientConfig

	private logger: ConsolaInstance

	/**
	 * Default configuration for retry behavior when network requests fail
	 * - maxRetries: Maximum number of retry attempts before failing
	 * - backoffMs: Initial backoff time in milliseconds (doubles with each retry)
	 */
	private defaultRetryConfig: RetryConfig = {
		maxRetries: 3,
		backoffMs: 1000,
	}

	/**
	 * Creates a new IndexerClient instance
	 */
	constructor(config: PartialClientConfig) {
		this.client = config.queryClient
		this.config = {
			pollInterval: DEFAULT_POLL_INTERVAL,
			...config,
		}
		this.logger = createConsola({
			level: LogLevels[config.tracing ? "trace" : "info"],
			formatOptions: {
				columns: 80,
				colors: true,
				compact: true,
				date: false,
			},
		})
	}

	/**
	 * Query for a single state machine update event greater than or equal to the given height.
	 * @params statemachineId - ID of the state machine
	 * @params height - Starting block height
	 * @params chain - The identifier for the chain where the state machine update should be queried (corresponds to a stateMachineId)
	 * @returns Closest state machine update
	 */
	async queryStateMachineUpdateByHeight({
		statemachineId,
		height,
		chain,
	}: {
		statemachineId: string
		chain: string
		height: number
	}): Promise<StateMachineUpdate | undefined> {
		const logger = this.logger.withTag("[queryStateMachineUpdateByHeight]()")
		const message = `querying StateMachineId(${statemachineId}) update by Height(${height}) in chain Chain(${chain})`

		const response = await this.withRetry(
			() => {
				return this.client.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_HEIGHT, {
					statemachineId,
					height,
					chain,
				})
			},
			{ logger: logger, logMessage: message },
		)

		const first_node = response?.stateMachineUpdateEvents?.nodes[0]
		if (first_node?.createdAt) {
			//@ts-ignore
			first_node.timestamp = Math.floor(dateStringtoTimestamp(first_node.createdAt) / 1000)
		}
		logger.trace("Response >", first_node)

		//@ts-ignore
		return first_node
	}

	/**
	 * Query for a single state machine update event greater than or equal to the given timestamp.
	 * @params statemachineId - ID of the state machine
	 * @params timestamp - Starting block timestamp
	 * @params chain - The identifier for the chain where the state machine update should be queried (corresponds to a stateMachineId)
	 * @returns Closest state machine update
	 */
	async queryStateMachineUpdateByTimestamp({
		statemachineId,
		commitmentTimestamp,
		chain,
	}: {
		statemachineId: string
		commitmentTimestamp: bigint
		chain: string
	}): Promise<StateMachineUpdate | undefined> {
		const logger = this.logger.withTag("[queryStateMachineUpdateByTimestamp]")
		const message = `querying StateMachineId(${statemachineId}) update by Timestamp(${commitmentTimestamp}) in Chain(${chain})`

		const response = await this.withRetry(
			() =>
				this.client.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_TIMESTAMP, {
					statemachineId,
					commitmentTimestamp: commitmentTimestamp.toString(),
					chain,
				}),
			{ logger, logMessage: message },
		)

		const first_node = response?.stateMachineUpdateEvents?.nodes[0]
		if (first_node?.createdAt) {
			//@ts-ignore
			first_node.timestamp = Math.floor(dateStringtoTimestamp(first_node.createdAt) / 1000)
		}
		logger.trace("Response >", first_node)

		//@ts-ignore
		return first_node
	}

	/**
	 * Queries a request by CommitmentHash
	 *
	 * @param commitment_hash - Can be commitment
	 * @returns Latest status and block metadata of the request
	 */
	async queryPostRequest(commitment_hash: string): Promise<PostRequestWithStatus | undefined> {
		return _queryRequestInternal({
			commitmentHash: commitment_hash,
			queryClient: this.client,
			logger: this.logger,
		})
	}

	/**
	 * Queries a request by any of its associated hashes and returns it alongside its statuses
	 * Statuses will be one of SOURCE, HYPERBRIDGE_DELIVERED and DESTINATION
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns Latest status and block metadata of the request
	 */
	async queryGetRequest(hash: string): Promise<GetRequestWithStatus | undefined> {
		return _queryGetRequestInternal({
			commitmentHash: hash,
			queryClient: this.client,
			logger: this.logger,
		})
	}

	/**
	 * Queries the response associated with a specific request ID and returns its commitment
	 * @param requestId - The ID of the request to find the associated response for
	 * @returns The response associated with the given request ID, or undefined if not found
	 */
	async queryResponseByRequestId(requestId: string): Promise<ResponseCommitmentWithValues | undefined> {
		const response = await this.withRetry(() =>
			this.client.request<GetResponseByRequestIdResponse>(GET_RESPONSE_BY_REQUEST_ID, {
				requestId,
			}),
		)

		// If no responses are found or nodes array is empty, return undefined
		if (!response.getResponses.nodes.length) return undefined

		// Return just the first response
		const firstResponse = response.getResponses.nodes[0]

		return {
			commitment: firstResponse.commitment,
			values: firstResponse.responseMessage,
		}
	}

	/**
	 * Enhances a request with finality events by querying state machine updates.
	 *
	 * This method augments a request object with additional inferred status events
	 * that represent chain finality confirmations. It adds:
	 * - SOURCE_FINALIZED: When the source chain has finalized the request
	 * - HYPERBRIDGE_FINALIZED: When Hyperbridge has finalized the delivery confirmation
	 *
	 * The method also generates appropriate calldata for submitting cross-chain proofs
	 * when applicable.
	 *
	 * @param request - The request to enhance with finality events
	 * @returns The request with finality events added
	 * @private
	 */
	private async addRequestFinalityEvents(request: PostRequestWithStatus): Promise<PostRequestWithStatus> {
		const events: RequestStatusWithMetadata[] = []

		const addFinalityEvents = (request: PostRequestWithStatus) => {
			this.logger.trace(`Added ${events.length} \`Request\` finality events`, events)

			request.statuses = [...request.statuses, ...events]
			return request
		}

		let hyperbridgeDelivered: RequestStatusWithMetadata | undefined

		if (request.source === this.config.hyperbridge.stateMachineId) {
			// the first status contains the blocknumber of the initial request
			hyperbridgeDelivered = request.statuses[0]
		} else {
			// we assume there's always a SOURCE event which contains the blocknumber of the initial request
			const sourceFinality = await this.queryStateMachineUpdateByHeight({
				statemachineId: request.source,
				height: request.statuses[0].metadata.blockNumber,
				chain: this.config.hyperbridge.stateMachineId,
			})

			// no finality event found, return request as is
			if (!sourceFinality) return addFinalityEvents(request)

			// Insert finality event into request.statuses at index 1
			events.push({
				status: RequestStatus.SOURCE_FINALIZED,
				metadata: {
					blockHash: sourceFinality.blockHash,
					blockNumber: sourceFinality.height,
					transactionHash: sourceFinality.transactionHash,
					timestamp: sourceFinality.timestamp,
				},
			})

			// check if there's a hyperbridge delivered event
			hyperbridgeDelivered = request.statuses.find((item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED)

			if (!hyperbridgeDelivered) return addFinalityEvents(request)
		}

		// no need to query finality event if destination is hyperbridge
		if (request.dest === this.config.hyperbridge.stateMachineId) {
			return addFinalityEvents(request)
		}

		const hyperbridgeFinality = await this.queryStateMachineUpdateByHeight({
			statemachineId: this.config.hyperbridge.stateMachineId,
			height: hyperbridgeDelivered.metadata.blockNumber,
			chain: request.dest,
		})

		if (!hyperbridgeFinality) return addFinalityEvents(request)

		// check if request receipt exists on destination chain
		const destChain = await getChain(this.config.dest)
		const hyperbridge = await getChain({
			...this.config.hyperbridge,
			hasher: "Keccak",
		})

		const proof = await hyperbridge.queryProof(
			{ Requests: [postRequestCommitment(request).commitment] },
			request.dest,
			BigInt(hyperbridgeFinality.height),
		)

		const calldata = destChain.encode({
			kind: "PostRequest",
			proof: {
				stateMachine: this.config.hyperbridge.stateMachineId,
				consensusStateId: this.config.hyperbridge.consensusStateId,
				proof,
				height: BigInt(hyperbridgeFinality.height),
			},
			requests: [request],
			signer: pad("0x"),
		})

		events.push({
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: hyperbridgeFinality.blockHash,
				blockNumber: hyperbridgeFinality.height,
				transactionHash: hyperbridgeFinality.transactionHash,
				timestamp: hyperbridgeFinality.timestamp,
				calldata,
			},
		})

		return addFinalityEvents(request)
	}

	/**
	 * Adds timeout finality events to a request by querying for relevant timeout proofs and
	 * chain state necessary for timeout processing.
	 *
	 * This method enhances a request object with additional status events related to the
	 * timeout flow, including:
	 * - PENDING_TIMEOUT: When a request has passed its timeout timestamp
	 * - DESTINATION_FINALIZED: When the destination chain has finalized the timeout timestamp
	 * - HYPERBRIDGE_FINALIZED_TIMEOUT: When hyperbridge has finalized the timeout state
	 *
	 * The method also generates appropriate calldata for submitting timeout proofs.
	 *
	 * @param request - Request to fill timeout events for
	 * @returns Request with timeout events filled in, including any proof calldata for timeout submissions
	 * @private
	 */
	private async addTimeoutFinalityEvents(request: PostRequestWithStatus): Promise<PostRequestWithStatus> {
		// check if request receipt exists on destination chain
		const destChain = await getChain(this.config.dest)
		const hyperbridge = await getChain({
			...this.config.hyperbridge,
			hasher: "Keccak",
		})
		const events: RequestStatusWithMetadata[] = []
		const commitment = postRequestCommitment(request).commitment
		const reciept = await destChain.queryRequestReceipt(commitment)
		const destTimestamp = await destChain.timestamp()

		const addTimeoutEvents = (req: PostRequestWithStatus) => {
			this.logger.trace(`Added ${events.length} timeout events`, events)
			request.statuses = [...req.statuses, ...events]
			return request
		}

		if (request.timeoutTimestamp === 0n) {
			// Early exit for requests with no timeout configured
			// This prevents unnecessary timeout processing and expensive chain queries
			// The events array is still empty at this point, so no timeout events are added
			return addTimeoutEvents(request)
		}

		// request not timed out
		if (reciept || request.timeoutTimestamp > destTimestamp) {
			return addTimeoutEvents(request)
		}

		const is_finished = request.statuses.find((item) => item.status === RequestStatus.DESTINATION)

		if (!is_finished) {
			events.push({
				status: TimeoutStatus.PENDING_TIMEOUT,
				metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
			})
		}

		const delivered = request.statuses.find((item) => {
			return item.status === RequestStatus.HYPERBRIDGE_DELIVERED
		})

		let hyperbridgeFinalized: StateMachineUpdate | undefined
		if (!delivered) {
			// either the request was never delivered to hyperbridge
			// or hyperbridge was the destination of the request
			hyperbridgeFinalized = await this.queryStateMachineUpdateByTimestamp({
				statemachineId: this.config.hyperbridge.stateMachineId,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: request.source,
			})
		} else {
			const destFinalized = await this.queryStateMachineUpdateByTimestamp({
				statemachineId: request.dest,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: this.config.hyperbridge.stateMachineId,
			})

			if (!destFinalized) return addTimeoutEvents(request)

			events.push({
				status: TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT,
				metadata: {
					blockHash: destFinalized.blockHash,
					blockNumber: destFinalized.blockNumber,
					transactionHash: destFinalized.transactionHash,
					timestamp: destFinalized.timestamp,
				},
			})

			// if the source is the hyperbridge state machine, no further action is needed
			// use the timeout stream to timeout on hyperbridge
			if (request.source === this.config.hyperbridge.stateMachineId) return request

			const hyperbridgeTimedOut = request.statuses.find(
				(item) => item.status === TimeoutStatus.HYPERBRIDGE_TIMED_OUT,
			)
			if (!hyperbridgeTimedOut) return addTimeoutEvents(request)
			hyperbridgeFinalized = await this.queryStateMachineUpdateByHeight({
				statemachineId: this.config.hyperbridge.stateMachineId,
				height: hyperbridgeTimedOut.metadata.blockNumber,
				chain: request.source,
			})
		}

		if (!hyperbridgeFinalized) return addTimeoutEvents(request)

		const proof = await hyperbridge.queryStateProof(BigInt(hyperbridgeFinalized.height), [
			hyperbridge.requestReceiptKey(commitment),
		])
		const sourceChain = await getChain(this.config.source)
		const calldata = sourceChain.encode({
			kind: "TimeoutPostRequest",
			proof: {
				proof,
				height: BigInt(hyperbridgeFinalized.height),
				stateMachine: this.config.hyperbridge.stateMachineId,
				consensusStateId: this.config.hyperbridge.consensusStateId,
			},
			requests: [
				{
					source: request.source,
					dest: request.dest,
					from: request.from,
					to: request.to,
					nonce: request.nonce,
					body: request.body,
					timeoutTimestamp: request.timeoutTimestamp,
				},
			],
		})

		events.push({
			status: TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT,
			metadata: {
				blockHash: hyperbridgeFinalized.blockHash,
				blockNumber: hyperbridgeFinalized.blockNumber,
				transactionHash: hyperbridgeFinalized.transactionHash,
				timestamp: hyperbridgeFinalized.timestamp,
				calldata,
			},
		})

		return addTimeoutEvents(request)
	}

	/**
	 * Queries a request returns it alongside its statuses,
	 * including any finalization events.
	 * @param hash - Commitment hash
	 * @returns Full request data with all inferred status events, including SOURCE_FINALIZED and HYPERBRIDGE_FINALIZED
	 * @remarks Unlike queryRequest(), this method adds derived finalization status events by querying state machine updates
	 */
	async queryRequestWithStatus(hash: string): Promise<PostRequestWithStatus | undefined> {
		let request = await this.queryPostRequest(hash)

		if (!request) return
		request = await this.addRequestFinalityEvents(request)
		request = await this.addTimeoutFinalityEvents(request)

		// ensure all statuses are sorted by weight
		request.statuses = request.statuses.sort(
			(a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status],
		)

		return request
	}

	/**
	 * Create a Stream of status updates for a post request.
	 * Stream ends when either the request reaches the destination or times out.
	 * If the stream yields TimeoutStatus.PENDING_TIMEOUT, use postRequestTimeoutStream() to begin timeout processing.
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 * @example
	 *
	 * let client = new IndexerClient(config)
	 * let stream = client.postRequestStatusStream(hash)
	 *
	 * // you can use a for-await-of loop
	 * for await (const status of stream) {
	 *   console.log(status)
	 * }
	 *
	 * // you can also use a while loop
	 * while (true) {
	 *   const status = await stream.next()
	 *   if (status.done) {
	 *     break
	 *   }
	 *   console.log(status.value)
	 * }
	 *
	 */
	async *postRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		const logger = this.logger.withTag("[postRequestStatusStream]")

		// wait for request to be created
		let request: PostRequestWithStatus | undefined

		while (!request) {
			await this.sleep_for_interval()
			request = await this.queryPostRequest(hash)
		}

		logger.trace("`Request` found")
		const chain = await getChain(this.config.dest)
		const timeoutStream =
			request.timeoutTimestamp > 0n ? this.timeoutStream(request.timeoutTimestamp, chain) : undefined
		const statusStream = this.postRequestStatusStreamInternal(hash)

		logger.trace("Listening for events")
		const combined = timeoutStream ? mergeRace(timeoutStream, statusStream) : statusStream

		logger.trace("Listening for events")
		let item = await combined.next()

		while (!item.done) {
			logger.trace(`Yielding Event(${item.value.status})`)

			yield item.value
			item = await combined.next()
		}

		logger.trace("Streaming complete")
		return
	}

	/*
	 * Returns a generator that will yield true if the request is timed out
	 * If the request does not have a timeout, it will never yield
	 * @param request - Request to timeout
	 */
	async *timeoutStream(timeoutTimestamp: bigint, chain: IChain): AsyncGenerator<RequestStatusWithMetadata, void> {
		const logger = this.logger.withTag("[timeoutStream()]")

		if (timeoutTimestamp > 0n) {
			let timestamp = await chain.timestamp()

			while (timestamp < timeoutTimestamp) {
				logger.trace("Comparing timeout timestamps", { control: timeoutTimestamp, latest: timestamp })

				const diff = BigInt(timeoutTimestamp) - BigInt(timestamp)
				await this.sleep_for(Number(diff))
				timestamp = await chain.timestamp()
			}

			yield {
				status: TimeoutStatus.PENDING_TIMEOUT,
				metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
			}

			return
		}
	}

	/**
	 * Create a Stream of status updates
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 */
	private async *postRequestStatusStreamInternal(hash: string): AsyncGenerator<RequestStatusWithMetadata, void> {
		let request: PostRequestWithStatus | undefined
		while (!request) {
			await this.sleep_for_interval()
			request = await this.queryPostRequest(hash)
		}

		let status: RequestStatusKey =
			request.source === this.config.hyperbridge.stateMachineId
				? RequestStatus.HYPERBRIDGE_DELIVERED
				: RequestStatus.SOURCE

		const latestMetadata = request.statuses[request.statuses.length - 1]

		const latest_request = maxBy(
			[status, latestMetadata.status as RequestStatusKey],
			(item) => REQUEST_STATUS_WEIGHTS[item],
		)

		if (!latest_request) return

		// start with the latest status
		status = latest_request

		while (true) {
			switch (status) {
				// request has been dispatched from source chain
				case RequestStatus.SOURCE: {
					let sourceUpdate: StateMachineUpdate | undefined
					while (!sourceUpdate) {
						await this.sleep_for_interval()
						sourceUpdate = await this.queryStateMachineUpdateByHeight({
							statemachineId: request.source,
							height: request.statuses[0].metadata.blockNumber,
							chain: this.config.hyperbridge.stateMachineId,
						})
					}

					yield {
						status: RequestStatus.SOURCE_FINALIZED,
						metadata: {
							blockHash: sourceUpdate.blockHash,
							blockNumber: sourceUpdate.height,
							transactionHash: sourceUpdate.transactionHash,
							timestamp: sourceUpdate.timestamp,
						},
					}

					status = RequestStatus.SOURCE_FINALIZED
					break
				}

				// finality proofs for request has been verified on Hyperbridge
				case RequestStatus.SOURCE_FINALIZED: {
					// wait for the request to be delivered on Hyperbridge
					while (!request || request.statuses.length < 2) {
						await this.sleep_for_interval()
						request = await this.queryPostRequest(hash)
					}

					status =
						request.dest === this.config.hyperbridge.stateMachineId
							? RequestStatus.DESTINATION
							: RequestStatus.HYPERBRIDGE_DELIVERED

					yield {
						status,
						metadata: {
							blockHash: request.statuses[1].metadata.blockHash,
							blockNumber: request.statuses[1].metadata.blockNumber,
							transactionHash: request.statuses[1].metadata.transactionHash,
							// @ts-ignore
							timestamp: request.statuses[1].metadata.timestamp,
						},
					}
					break
				}

				// the request has been verified and aggregated on Hyperbridge
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					// Get the latest state machine update for hyperbridge on the destination chain
					let hyperbridgeFinalized: StateMachineUpdate | undefined
					const index = request.source === this.config.hyperbridge.stateMachineId ? 0 : 1
					while (!hyperbridgeFinalized) {
						await this.sleep_for_interval()
						hyperbridgeFinalized = await this.queryStateMachineUpdateByHeight({
							statemachineId: this.config.hyperbridge.stateMachineId,
							height: request.statuses[index].metadata.blockNumber,
							chain: request.dest,
						})
					}

					const destChain = await getChain(this.config.dest)
					const hyperbridge = await getChain({
						...this.config.hyperbridge,
						hasher: "Keccak",
					})

					const proof = await hyperbridge.queryProof(
						{ Requests: [postRequestCommitment(request).commitment] },
						request.dest,
						BigInt(hyperbridgeFinalized.height),
					)

					const calldata = destChain.encode({
						kind: "PostRequest",
						proof: {
							stateMachine: this.config.hyperbridge.stateMachineId,
							consensusStateId: this.config.hyperbridge.consensusStateId,
							proof,
							height: BigInt(hyperbridgeFinalized.height),
						},
						requests: [request],
						signer: pad("0x"),
					})

					const { stateId } = parseStateMachineId(this.config.hyperbridge.stateMachineId)

					await waitForChallengePeriod(destChain, {
						height: BigInt(hyperbridgeFinalized.height),
						id: {
							stateId,
							consensusStateId: toHex(this.config.hyperbridge.consensusStateId),
						},
					})

					yield {
						status: RequestStatus.HYPERBRIDGE_FINALIZED,
						metadata: {
							blockHash: hyperbridgeFinalized.blockHash,
							blockNumber: hyperbridgeFinalized.height,
							transactionHash: hyperbridgeFinalized.transactionHash,
							timestamp: hyperbridgeFinalized.timestamp,
							calldata,
						},
					}
					status = RequestStatus.HYPERBRIDGE_FINALIZED
					break
				}

				// request has been finalized by hyperbridge
				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					// wait for the request to be delivered to the destination
					let delivered = request.statuses.find((s) => s.status === RequestStatus.DESTINATION)
					while (!request || !delivered) {
						await this.sleep_for_interval()
						request = await this.queryPostRequest(hash)
						delivered = request?.statuses.find((s) => s.status === RequestStatus.DESTINATION)
					}

					const index = request.source === this.config.hyperbridge.stateMachineId ? 1 : 2

					yield {
						status: RequestStatus.DESTINATION,
						metadata: {
							blockHash: request.statuses[index].metadata.blockHash,
							blockNumber: request.statuses[index].metadata.blockNumber,
							transactionHash: request.statuses[index].metadata.transactionHash,
							// @ts-ignore
							timestamp: request.statuses[index].metadata.timestamp,
						},
					}
					status = RequestStatus.DESTINATION
					break
				}

				case RequestStatus.DESTINATION:
					return
			}
		}
	}

	private sleep_for(duration: number): Promise<void> {
		this.logger.trace(`Sleeping for ${duration}ms`)
		return sleep(duration)
	}

	private sleep_for_interval(): Promise<void> {
		return this.sleep_for(this.config.pollInterval)
	}

	/**
	 * Create a Stream of status updates for a get request.
	 * Stream ends when either the request reaches the destination or times out.
	 * If the stream yields TimeoutStatus.PENDING_TIMEOUT, use postRequestTimeoutStream() to begin timeout processing.
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 * @example
	 *
	 * let client = new IndexerClient(config)
	 * let stream = client.getRequestStatusStream(hash)
	 *
	 * // you can use a for-await-of loop
	 * for await (const status of stream) {
	 *   console.log(status)
	 * }
	 *
	 * // you can also use a while loop
	 * while (true) {
	 *   const status = await stream.next()
	 *   if (status.done) {
	 *     break
	 *   }
	 *   console.log(status.value)
	 * }
	 *
	 */
	async *getRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		// wait for request to be created
		let request: GetRequestWithStatus | undefined
		while (!request) {
			await this.sleep_for_interval()
			request = await this.queryGetRequest(hash)
		}

		const chain = await getChain(this.config.dest)
		const timeoutStream =
			request.timeoutTimestamp > 0n ? this.timeoutStream(request.timeoutTimestamp, chain) : undefined
		const statusStream = this.getRequestStatusStreamInternal(hash)
		const combined = timeoutStream ? mergeRace(timeoutStream, statusStream) : statusStream

		let item = await combined.next()
		while (!item.done) {
			yield item.value
			item = await combined.next()
		}
		return
	}

	/**
	 * Create a Stream of status updates
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 */
	private async *getRequestStatusStreamInternal(hash: string): AsyncGenerator<RequestStatusWithMetadata, void> {
		let request: GetRequestWithStatus | undefined
		while (!request) {
			await this.sleep_for_interval()
			request = await this.queryGetRequest(hash)
		}

		let status: RequestStatusKey | undefined =
			request.source === this.config.hyperbridge.stateMachineId
				? RequestStatus.HYPERBRIDGE_DELIVERED
				: RequestStatus.SOURCE

		const latestMetadata = request.statuses[request.statuses.length - 1]

		// start with the latest status
		status = maxBy([status, latestMetadata.status as RequestStatusKey], (item) => REQUEST_STATUS_WEIGHTS[item])

		if (!status) return

		while (true) {
			switch (status) {
				// request has been dispatched from source chain
				case RequestStatus.SOURCE: {
					let sourceUpdate: StateMachineUpdate | undefined
					while (!sourceUpdate) {
						await this.sleep_for_interval()
						sourceUpdate = await this.queryStateMachineUpdateByHeight({
							statemachineId: request.source,
							height: request.statuses[0].metadata.blockNumber,
							chain: this.config.hyperbridge.stateMachineId,
						})
					}

					yield {
						status: RequestStatus.SOURCE_FINALIZED,
						metadata: {
							blockHash: sourceUpdate.blockHash,
							blockNumber: sourceUpdate.height,
							transactionHash: sourceUpdate.transactionHash,
							timestamp: sourceUpdate.timestamp,
						},
					}
					status = RequestStatus.SOURCE_FINALIZED
					break
				}

				// finality proofs for request has been verified on Hyperbridge
				case RequestStatus.SOURCE_FINALIZED: {
					// wait for the request to be delivered on Hyperbridge
					while (!request || request.statuses.length < 2) {
						await this.sleep_for_interval()
						request = await this.queryGetRequest(hash)
					}

					status =
						request.source === this.config.hyperbridge.stateMachineId
							? RequestStatus.DESTINATION
							: RequestStatus.HYPERBRIDGE_DELIVERED

					yield {
						status,
						metadata: {
							blockHash: request.statuses[1].metadata.blockHash,
							blockNumber: request.statuses[1].metadata.blockNumber,
							transactionHash: request.statuses[1].metadata.transactionHash,
							// @ts-ignore
							timestamp: request.statuses[1].metadata.timestamp,
						},
					}
					break
				}

				// the request has been verified and aggregated on Hyperbridge
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					// If Hyperbridge was the source, the request is already complete
					if (request.source === this.config.hyperbridge.stateMachineId) {
						return
					}
					// Get the latest state machine update for hyperbridge on the destination chain
					let hyperbridgeFinalized: StateMachineUpdate | undefined
					while (!hyperbridgeFinalized) {
						await this.sleep_for_interval()
						hyperbridgeFinalized = await this.queryStateMachineUpdateByHeight({
							statemachineId: this.config.hyperbridge.stateMachineId,
							height: request.statuses[1].metadata.blockNumber,
							chain: request.source,
						})
					}

					const sourceChain = await getChain(this.config.source)
					const hyperbridge = await getChain({
						...this.config.hyperbridge,
						hasher: "Keccak",
					})

					const response = await this.queryResponseByRequestId(hash)

					const proof = await hyperbridge.queryProof(
						{ Responses: [response?.commitment as HexString] },
						request.source,
						BigInt(hyperbridgeFinalized.height),
					)

					const calldata = sourceChain.encode({
						kind: "GetResponse",
						proof: {
							stateMachine: this.config.hyperbridge.stateMachineId,
							consensusStateId: this.config.hyperbridge.consensusStateId,
							proof,
							height: BigInt(hyperbridgeFinalized.height),
						},
						responses: [
							{
								get: request,
								values: request.keys.map((key, index) => ({
									key,
									value: (response?.values[index] as HexString) || "0x",
								})),
							},
						],
						signer: pad("0x"),
					})

					yield {
						status: RequestStatus.HYPERBRIDGE_FINALIZED,
						metadata: {
							blockHash: hyperbridgeFinalized.blockHash,
							blockNumber: hyperbridgeFinalized.height,
							transactionHash: hyperbridgeFinalized.transactionHash,
							timestamp: hyperbridgeFinalized.timestamp,
							calldata,
						},
					}
					status = RequestStatus.HYPERBRIDGE_FINALIZED
					break
				}

				// request has been finalized by hyperbridge
				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					// If Hyperbridge was the source, the request is already complete
					if (request.source === this.config.hyperbridge.stateMachineId) {
						return
					}

					// wait for the request to be delivered to the destination
					let delivered = request.statuses.find((s) => s.status === RequestStatus.DESTINATION)
					while (!request || !delivered) {
						await this.sleep_for_interval()
						request = await this.queryGetRequest(hash)
						delivered = request?.statuses.find((s) => s.status === RequestStatus.DESTINATION)
					}

					yield {
						status: RequestStatus.DESTINATION,
						metadata: {
							blockHash: request.statuses[2].metadata.blockHash,
							blockNumber: request.statuses[2].metadata.blockNumber,
							transactionHash: request.statuses[2].metadata.transactionHash,
							//@ts-ignore
							timestamp: request.statuses[2].metadata.timestamp,
						},
					}
					status = RequestStatus.DESTINATION
					break
				}

				case RequestStatus.DESTINATION:
					return
			}
		}
	}

	/**
	 * Create a Stream of status updates for a timed out post request.
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 * @example
	 *
	 * let client = new IndexerClient(config)
	 * let stream = client.postRequestTimeoutStream(hash)
	 *
	 * // you can use a for-await-of loop
	 * for await (const status of stream) {
	 *   console.log(status)
	 * }
	 *
	 * // you can also use a while loop
	 * while (true) {
	 *   const status = await stream.next()
	 *   if (status.done) {
	 *     break
	 *   }
	 *   console.log(status.value)
	 * }
	 */
	async *postRequestTimeoutStream(hash: HexString): AsyncGenerator<PostRequestTimeoutStatus, void> {
		const logger = this.logger.withTag("PostRequestTimeoutStream")
		const request = await this.queryPostRequest(hash)
		if (!request) throw new Error("Request not found")

		logger.trace("Reading destination chain")
		const destChain = await getChain(this.config.dest)

		// if the destination is hyperbridge, then just wait for hyperbridge finality
		let status: TimeoutStatusKey =
			request.dest === this.config.hyperbridge.stateMachineId
				? TimeoutStatus.HYPERBRIDGE_TIMED_OUT
				: TimeoutStatus.PENDING_TIMEOUT

		const commitment = postRequestCommitment(request).commitment
		const hyperbridge = (await getChain({
			...this.config.hyperbridge,
			hasher: "Keccak",
		})) as unknown as SubstrateChain

		const latest = request.statuses[request.statuses.length - 1]
		const latest_request = maxBy(
			[status, latest.status as TimeoutStatusKey],
			(item) => TIMEOUT_STATUS_WEIGHTS[item],
		)

		logger.trace(`Reading lastest request: ${latest_request}`)

		if (!latest_request) {
			logger.trace("Ending stream. Latest request not found")
			return
		}

		// we're always interested in the latest status
		status = latest_request

		while (true) {
			switch (status) {
				case TimeoutStatus.PENDING_TIMEOUT: {
					const receipt = await hyperbridge.queryRequestReceipt(commitment)
					if (!receipt && request.source !== this.config.hyperbridge.stateMachineId) {
						status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
						break
					}

					let update: StateMachineUpdate | undefined
					while (!update) {
						await this.sleep_for_interval()
						update = await this.queryStateMachineUpdateByTimestamp({
							statemachineId: request.dest,
							commitmentTimestamp: request.timeoutTimestamp,
							chain: this.config.hyperbridge.stateMachineId,
						})
					}

					yield {
						status: TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT,
						metadata: {
							blockHash: update.blockHash,
							blockNumber: update.height,
							transactionHash: update.transactionHash,
							timestamp: update.timestamp,
						},
					}
					status = TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT
					break
				}

				case TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT: {
					if (request.source !== this.config.hyperbridge.stateMachineId) {
						const receipt = await hyperbridge.queryRequestReceipt(commitment)
						if (!receipt) {
							status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
							break
						}
					}

					const update = (await this.queryStateMachineUpdateByTimestamp({
						statemachineId: request.dest,
						commitmentTimestamp: request.timeoutTimestamp,
						chain: this.config.hyperbridge.stateMachineId,
					}))!

					const proof = await destChain.queryStateProof(BigInt(update.height), [
						destChain.requestReceiptKey(commitment),
					])

					const { stateId } = parseStateMachineId(request.dest)

					await waitForChallengePeriod(hyperbridge, {
						height: BigInt(update.height),
						id: {
							stateId,
							consensusStateId: toHex(this.config.dest.consensusStateId),
						},
					})

					const { blockHash, transactionHash, blockNumber, timestamp } = await hyperbridge.submitUnsigned({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: request.dest,
							consensusStateId: this.config.dest.consensusStateId,
						},
						requests: [
							{
								source: request.source,
								dest: request.dest,
								from: request.from,
								to: request.to,
								nonce: request.nonce,
								body: request.body,
								timeoutTimestamp: request.timeoutTimestamp,
							},
						],
					})

					status =
						request.source === this.config.hyperbridge.stateMachineId
							? TimeoutStatus.TIMED_OUT
							: TimeoutStatus.HYPERBRIDGE_TIMED_OUT

					yield {
						status,
						metadata: {
							blockHash,
							transactionHash,
							blockNumber,
							timestamp,
						},
					}
					break
				}

				case TimeoutStatus.HYPERBRIDGE_TIMED_OUT: {
					const hasDelivered = request.statuses.some(
						(item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED,
					)
					let update: StateMachineUpdate | undefined
					if (!hasDelivered) {
						// if request was never delivered to Hyperbridge
						// then query for any state machine update > requestTimestamp
						while (!update) {
							await this.sleep_for_interval()
							update = await this.queryStateMachineUpdateByTimestamp({
								statemachineId: this.config.hyperbridge.stateMachineId,
								commitmentTimestamp: request.timeoutTimestamp,
								chain: request.source,
							})
						}
					} else {
						let timeout: RequestStatusWithMetadata | undefined
						while (!timeout || timeout?.status !== TimeoutStatus.HYPERBRIDGE_TIMED_OUT) {
							await this.sleep_for_interval()
							const req = await this.queryPostRequest(hash)
							if (!req) continue
							timeout = req.statuses
								.sort((a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status])
								.pop()
						}

						while (!update) {
							await this.sleep_for_interval()
							update = await this.queryStateMachineUpdateByHeight({
								statemachineId: this.config.hyperbridge.stateMachineId,
								height: timeout.metadata.blockNumber,
								chain: request.source,
							})
						}
					}

					const proof = await hyperbridge.queryStateProof(BigInt(update.height), [
						hyperbridge.requestReceiptKey(commitment),
					])

					const sourceChain = await getChain(this.config.source)
					const calldata = sourceChain.encode({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: this.config.hyperbridge.stateMachineId,
							consensusStateId: this.config.hyperbridge.consensusStateId,
						},
						requests: [
							{
								source: request.source,
								dest: request.dest,
								from: request.from,
								to: request.to,
								nonce: request.nonce,
								body: request.body,
								timeoutTimestamp: request.timeoutTimestamp,
							},
						],
					})

					const { stateId } = parseStateMachineId(this.config.hyperbridge.stateMachineId)

					await waitForChallengePeriod(sourceChain, {
						height: BigInt(update.height),
						id: {
							stateId,
							consensusStateId: toHex(this.config.hyperbridge.consensusStateId),
						},
					})

					yield {
						status: TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT,
						metadata: {
							transactionHash: update.transactionHash,
							blockNumber: update.blockNumber,
							blockHash: update.blockHash,
							timestamp: update.timestamp,
							calldata,
						},
					}
					status = TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT
					break
				}

				case TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT: {
					// wait for the request to be timed out on the source
					let req = await this.queryPostRequest(hash)
					let delivered = req?.statuses.find((s) => s.status === RequestStatus.TIMED_OUT)
					while (!req || !delivered) {
						await this.sleep_for_interval()
						req = await this.queryPostRequest(hash)
						delivered = req?.statuses.find((s) => s.status === RequestStatus.TIMED_OUT)
					}
					yield {
						status: TimeoutStatus.TIMED_OUT,
						metadata: {
							transactionHash: delivered.metadata.transactionHash,
							blockNumber: delivered.metadata.blockNumber,
							blockHash: delivered.metadata.blockHash,
							timestamp: delivered.metadata.timestamp,
						},
					}
					status = TimeoutStatus.TIMED_OUT
					break
				}

				case TimeoutStatus.TIMED_OUT:
					return
			}
		}
	}

	/**
	 * Query for asset teleported events by sender, recipient, and destination chain
	 * @param from - The sender address
	 * @param to - The recipient address
	 * @param dest - The destination chain ID
	 * @returns The asset teleported event if found, undefined otherwise
	 */
	async queryAssetTeleported(
		from: string,
		to: string,
		dest: string,
		blockNumber: number,
	): Promise<AssetTeleported | undefined> {
		const response = await this.withRetry(() =>
			this.client.request<AssetTeleportedResponse>(ASSET_TELEPORTED_BY_PARAMS, {
				from,
				to,
				dest,
				blockNumber,
			}),
		)

		return response.assetTeleporteds.nodes[0]
	}

	/**
	 * Executes an async operation with exponential backoff retry
	 * @param operation - Async function to execute
	 * @param retryConfig - Optional retry configuration
	 * @returns Result of the operation
	 * @throws Last encountered error after all retries are exhausted
	 *
	 * @example
	 * const result = await this.withRetry(() => this.queryStatus(hash));
	 */
	private async withRetry<T>(operation: () => Promise<T>, retryConfig: Partial<RetryConfig> = {}): Promise<T> {
		return retryPromise(operation, {
			...this.defaultRetryConfig,
			...retryConfig,
		})
	}

	/**
	 * Query for an order by its commitment hash
	 * @param commitment - The commitment hash of the order
	 * @returns The order with its status if found, undefined otherwise
	 */
	async queryOrder(commitment: HexString): Promise<OrderWithStatus | undefined> {
		return _queryOrderInternal({
			commitmentHash: commitment,
			queryClient: this.client,
			logger: this.logger,
		})
	}

	/**
	 * Create a Stream of status updates for an order.
	 * Stream ends when the order reaches a terminal state (FILLED, REDEEMED, or REFUNDED).
	 * @param commitment - The commitment hash of the order
	 * @returns AsyncGenerator that emits status updates until a terminal state is reached
	 * @example
	 *
	 * let client = new IndexerClient(config)
	 * let stream = client.orderStatusStream(commitment)
	 *
	 * // you can use a for-await-of loop
	 * for await (const status of stream) {
	 *   console.log(status)
	 * }
	 *
	 * // you can also use a while loop
	 * while (true) {
	 *   const status = await stream.next()
	 *   if (status.done) {
	 *     break
	 *   }
	 *   console.log(status.value)
	 * }
	 */
	async *orderStatusStream(commitment: HexString): AsyncGenerator<
		{
			status: OrderStatus
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp: bigint
				filler?: string
			}
		},
		void
	> {
		const logger = this.logger.withTag("[orderStatusStream]")

		let order: OrderWithStatus | undefined

		while (!order) {
			await this.sleep_for_interval()
			order = await _queryOrderInternal({
				commitmentHash: commitment,
				queryClient: this.client,
				logger: this.logger,
			})
		}

		logger.trace("`Order` found")
		// Yield initial status
		const latestStatus = order.statuses[order.statuses.length - 1]
		yield {
			status: latestStatus.status,
			metadata: latestStatus.metadata,
		}

		// If we're already in a terminal state, end the stream
		if ([OrderStatus.FILLED, OrderStatus.REDEEMED, OrderStatus.REFUNDED].includes(latestStatus.status)) {
			return
		}

		while (true) {
			await this.sleep_for_interval()
			const updatedOrder = await _queryOrderInternal({
				commitmentHash: commitment,
				queryClient: this.client,
				logger: this.logger,
			})

			if (!updatedOrder) continue

			const newLatestStatus = updatedOrder.statuses[updatedOrder.statuses.length - 1]

			if (newLatestStatus.status !== latestStatus.status) {
				yield {
					status: newLatestStatus.status,
					metadata: newLatestStatus.metadata,
				}

				if ([OrderStatus.FILLED, OrderStatus.REDEEMED, OrderStatus.REFUNDED].includes(newLatestStatus.status)) {
					return
				}
			}
		}
	}

	async *tokenGatewayAssetTeleportedStatusStream(commitment: HexString): AsyncGenerator<
		{
			status: TeleportStatus
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp: bigint
			}
		},
		void
	> {
		const logger = this.logger.withTag("[tokenGatewayAssetTeleportedStatusStream]")
		logger.trace(`Starting stream for token gateway asset teleported with commitment ${commitment}`)

		let lastStatus: TeleportStatus | undefined
		let lastBlockNumber: number | undefined

		while (true) {
			try {
				const teleport = await this.queryTokenGatewayAssetTeleported(commitment)
				if (!teleport) {
					logger.trace("No teleport found, waiting...")
					await this.sleep_for_interval()
					continue
				}

				const statuses = teleport.statuses
				if (statuses.length === 0) {
					logger.trace("No statuses found, waiting...")
					await this.sleep_for_interval()
					continue
				}

				// Find the latest status that we haven't seen yet
				const latestStatus = statuses[statuses.length - 1]
				if (lastStatus === latestStatus.status && lastBlockNumber === latestStatus.metadata.blockNumber) {
					logger.trace("No new status, waiting...")
					await this.sleep_for_interval()
					continue
				}

				lastStatus = latestStatus.status
				lastBlockNumber = latestStatus.metadata.blockNumber

				yield latestStatus

				// If we've reached a final status, end the stream
				if (
					latestStatus.status === TeleportStatus.RECEIVED ||
					latestStatus.status === TeleportStatus.REFUNDED
				) {
					logger.trace("Final status reached, ending stream")
					break
				}

				await this.sleep_for_interval()
			} catch (error) {
				logger.error("Error in token gateway asset teleported status stream:", error)
				await this.sleep_for_interval()
			}
		}
	}

	private async queryTokenGatewayAssetTeleported(
		commitment: HexString,
	): Promise<TokenGatewayAssetTeleportedWithStatus | undefined> {
		return _queryTokenGatewayAssetTeleportedInternal({
			commitmentHash: commitment,
			queryClient: this.client,
			logger: this.logger,
		})
	}

	/**
	 * Aggregate transactions with commitment.
	 * @param commitment
	 * @returns an object containing the transaction hash, block hash, block number, timestamp.
	 */
	async aggregateTransactionWithCommitment(
		commitment: HexString,
	): Promise<Awaited<ReturnType<SubstrateChain["submitUnsigned"]>>> {
		const logger = this.logger.withTag("aggregateTransactionWithCommitment")

		const { stateMachineId, consensusStateId } = this.config.source

		// check if request receipt exists on source chain
		const sourceChain = await getChain(this.config.source)
		const hyperbridge = (await getChain({
			...this.config.hyperbridge,
			hasher: "Keccak",
		})) as SubstrateChain

		logger.trace("Querying post request with commitment hash")
		const request = await this.queryPostRequest(commitment)
		if (!request) throw new Error("Request not found")

		logger.trace("Fetch latest stateMachineHeight")
		const latestStateMachineHeight = await hyperbridge.latestStateMachineHeight({
			stateId: parseStateMachineId(stateMachineId).stateId,
			consensusStateId: consensusStateId as HexString,
		})

		logger.trace("Query Request Proof from sourceChain")
		const proof = await sourceChain.queryProof(
			{ Requests: [commitment] },
			this.config.hyperbridge.stateMachineId,
			latestStateMachineHeight,
		)

		logger.trace("Construct Extrinsic and Submit Unsigned")
		const calldata = await hyperbridge.submitUnsigned({
			kind: "PostRequest",
			proof: {
				stateMachine: this.config.source.stateMachineId,
				consensusStateId: this.config.source.consensusStateId,
				proof,
				height: BigInt(latestStateMachineHeight),
			},
			requests: [
				{
					source: request.source,
					dest: request.dest,
					from: request.from,
					to: request.to,
					nonce: request.nonce,
					body: request.body,
					timeoutTimestamp: request.timeoutTimestamp,
				},
			],
			signer: pad("0x"),
		})

		return calldata
	}
}

interface PartialClientConfig extends Omit<ClientConfig, "pollInterval"> {
	pollInterval?: number
}
