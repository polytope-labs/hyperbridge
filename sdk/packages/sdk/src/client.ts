import { GraphQLClient } from "graphql-request"
import maxBy from "lodash/maxBy"
import { pad } from "viem"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import {
	RequestStatus,
	StateMachineUpdate,
	RequestResponse,
	StateMachineResponse,
	ClientConfig,
	RetryConfig,
	RequestWithStatus,
	HexString,
	TimeoutStatus,
	PostRequestTimeoutStatus,
	RequestStatusWithMetadata,
	AssetTeleported,
	AssetTeleportedResponse,
} from "@/types"
import {
	REQUEST_STATUS,
	STATE_MACHINE_UPDATES_BY_HEIGHT,
	STATE_MACHINE_UPDATES_BY_TIMESTAMP,
	ASSET_TELEPORTED_BY_PARAMS,
} from "@/queries"
import {
	COMBINED_STATUS_WEIGHTS,
	REQUEST_STATUS_WEIGHTS,
	TIMEOUT_STATUS_WEIGHTS,
	postRequestCommitment,
	sleep,
} from "@/utils"
import { getChain, IChain, SubstrateChain } from "@/chain"

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
	private client: GraphQLClient

	/**
	 * Configuration for the IndexerClient including URLs, poll intervals, and chain-specific settings
	 */
	private config: ClientConfig

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
	constructor(config: ClientConfig) {
		this.client = new GraphQLClient(config?.url || "http://localhost:3000/graphql")
		this.config = config
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
		const response = await this.withRetry(() =>
			this.client.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_HEIGHT, {
				statemachineId,
				height,
				chain,
			}),
		)

		return response.stateMachineUpdateEvents.nodes[0]
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
		const response = await this.withRetry(() =>
			this.client.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_TIMESTAMP, {
				statemachineId,
				commitmentTimestamp: commitmentTimestamp.toString(),
				chain,
			}),
		)

		return response.stateMachineUpdateEvents.nodes[0]
	}

	/**
	 * Queries a request by any of its associated hashes and returns it alongside its statuses
	 * Statuses will be one of SOURCE, HYPERBRIDGE_DELIVERED and DESTINATION
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns Latest status and block metadata of the request
	 */
	async queryRequest(hash: string): Promise<RequestWithStatus | undefined> {
		const self = this
		const response = await self.withRetry(() =>
			self.client.request<RequestResponse>(REQUEST_STATUS, {
				hash,
			}),
		)

		if (!response.requests.nodes[0]) return

		const statuses = response.requests.nodes[0].statusMetadata.nodes.map((item) => ({
			status: item.status as any,
			metadata: {
				blockHash: item.blockHash,
				blockNumber: parseInt(item.blockNumber),
				transactionHash: item.transactionHash,
			},
		}))

		// sort by ascending order
		const sorted = statuses.sort(
			(a, b) =>
				REQUEST_STATUS_WEIGHTS[a.status as RequestStatus] - REQUEST_STATUS_WEIGHTS[b.status as RequestStatus],
		)

		const request: RequestWithStatus = {
			...response.requests.nodes[0],
			statuses: sorted,
		}

		// @ts-ignore
		delete request.statusMetadata

		return request
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
	private async addRequestFinalityEvents(request: RequestWithStatus): Promise<RequestWithStatus> {
		const self = this

		let hyperbridgeDelivered: RequestStatusWithMetadata | undefined
		if (request.source === self.config.hyperbridge.stateMachineId) {
			// the first status contains the blocknumber of the initial request
			hyperbridgeDelivered = request.statuses[0]
		} else {
			// we assume there's always a SOURCE event which contains the blocknumber of the initial request
			const sourceFinality = await self.queryStateMachineUpdateByHeight({
				statemachineId: request.source,
				height: request.statuses[0].metadata.blockNumber,
				chain: self.config.hyperbridge.stateMachineId,
			})

			// no finality event found, return request as is
			if (!sourceFinality) return request

			// Insert finality event into request.statuses at index 1
			request.statuses.push({
				status: RequestStatus.SOURCE_FINALIZED,
				metadata: {
					blockHash: sourceFinality.blockHash,
					blockNumber: sourceFinality.height,
					transactionHash: sourceFinality.transactionHash,
				},
			})

			// check if there's a hyperbridge delivered event
			hyperbridgeDelivered = request.statuses.find((item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED)
			if (!hyperbridgeDelivered) return request
		}

		// no need to query finality event if destination is hyperbridge
		if (request.dest === self.config.hyperbridge.stateMachineId) return request

		const hyperbridgeFinality = await self.queryStateMachineUpdateByHeight({
			statemachineId: self.config.hyperbridge.stateMachineId,
			height: hyperbridgeDelivered.metadata.blockNumber,
			chain: request.dest,
		})
		if (!hyperbridgeFinality) return request

		// check if request receipt exists on destination chain
		const destChain = await getChain(self.config.dest)
		const hyperbridge = await getChain({
			...self.config.hyperbridge,
			hasher: "Keccak",
		})

		const proof = await hyperbridge.queryRequestsProof(
			[postRequestCommitment(request)],
			request.dest,
			BigInt(hyperbridgeFinality.height),
		)

		const calldata = destChain.encode({
			kind: "PostRequest",
			proof: {
				stateMachine: self.config.hyperbridge.stateMachineId,
				consensusStateId: self.config.hyperbridge.consensusStateId,
				proof,
				height: BigInt(hyperbridgeFinality.height),
			},
			requests: [request],
			signer: pad("0x"),
		})

		request.statuses.push({
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: hyperbridgeFinality.blockHash,
				blockNumber: hyperbridgeFinality.height,
				transactionHash: hyperbridgeFinality.transactionHash,
				calldata,
			},
		})

		return request
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
	private async addTimeoutFinalityEvents(request: RequestWithStatus): Promise<RequestWithStatus> {
		const self = this

		// check if request receipt exists on destination chain
		const destChain = await getChain(self.config.dest)
		const hyperbridge = await getChain({
			...self.config.hyperbridge,
			hasher: "Keccak",
		})

		const commitment = postRequestCommitment(request)
		const reciept = await destChain.queryRequestReceipt(commitment)
		const destTimestamp = await destChain.timestamp()

		// request not timed out
		if (reciept || request.timeoutTimestamp > destTimestamp) return request

		request.statuses.push({
			status: TimeoutStatus.PENDING_TIMEOUT,
			metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
		})

		const delivered = request.statuses.find((item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED)
		let hyperbridgeFinalized: StateMachineUpdate | undefined
		if (!delivered) {
			// either the request was never delivered to hyperbridge
			// or hyperbridge was the destination of the request
			hyperbridgeFinalized = await self.queryStateMachineUpdateByTimestamp({
				statemachineId: self.config.hyperbridge.stateMachineId,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: request.source,
			})
		} else {
			let destFinalized = await self.queryStateMachineUpdateByTimestamp({
				statemachineId: request.dest,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: self.config.hyperbridge.stateMachineId,
			})
			if (!destFinalized) return request

			request.statuses.push({
				status: TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT,
				metadata: {
					blockHash: destFinalized.blockHash,
					blockNumber: destFinalized.blockNumber,
					transactionHash: destFinalized.transactionHash,
				},
			})

			// if the source is the hyperbridge state machine, no further action is needed
			// use the timeout stream to timeout on hyperbridge
			if (request.source === self.config.hyperbridge.stateMachineId) return request

			const hyperbridgeTimedOut = request.statuses.find(
				(item) => item.status === TimeoutStatus.HYPERBRIDGE_TIMED_OUT,
			)
			if (!hyperbridgeTimedOut) return request
			hyperbridgeFinalized = await self.queryStateMachineUpdateByHeight({
				statemachineId: self.config.hyperbridge.stateMachineId,
				height: hyperbridgeTimedOut.metadata.blockNumber,
				chain: request.source,
			})
		}

		if (!hyperbridgeFinalized) return request

		const proof = await hyperbridge.queryStateProof(BigInt(hyperbridgeFinalized.height), [
			hyperbridge.requestReceiptKey(commitment),
		])
		const sourceChain = await getChain(self.config.source)
		let calldata = sourceChain.encode({
			kind: "TimeoutPostRequest",
			proof: {
				proof,
				height: BigInt(hyperbridgeFinalized.height),
				stateMachine: self.config.hyperbridge.stateMachineId,
				consensusStateId: self.config.hyperbridge.consensusStateId,
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

		request.statuses.push({
			status: TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT,
			metadata: {
				blockHash: hyperbridgeFinalized.blockHash,
				blockNumber: hyperbridgeFinalized.blockNumber,
				transactionHash: hyperbridgeFinalized.transactionHash,
				calldata,
			},
		})

		return request
	}

	/**
	 * Queries a request by any of its associated hashes and returns it alongside its statuses,
	 * including any finalization events.
	 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
	 * @returns Full request data with all inferred status events, including SOURCE_FINALIZED and HYPERBRIDGE_FINALIZED
	 * @remarks Unlike queryRequest(), this method adds derived finalization status events by querying state machine updates
	 */
	async queryRequestWithStatus(hash: string): Promise<RequestWithStatus | undefined> {
		let request = await this.queryRequest(hash)

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
		const self = this

		// wait for request to be created
		let request: RequestWithStatus | undefined
		while (!request) {
			await sleep(self.config.pollInterval)
			request = await self.queryRequest(hash)
			continue
		}

		const chain = await getChain(self.config.dest)
		const timeoutStream = self.timeoutStream(request.timeoutTimestamp, chain)
		const statusStream = self.postRequestStatusStreamInternal(hash)
		const combined = mergeRace(timeoutStream, statusStream)

		let item = await combined.next()
		while (!item.done) {
			yield item.value
			item = await combined.next()
		}
		return
	}

	/*
	 * Returns a generator that will yield true if the request is timed out
	 * If the request does not have a timeout, it will yield never yield
	 * @param request - Request to timeout
	 */
	async *timeoutStream(timeoutTimestamp: bigint, chain: IChain): AsyncGenerator<RequestStatusWithMetadata, void> {
		if (timeoutTimestamp > 0) {
			let timestamp = await chain.timestamp()
			while (timestamp < timeoutTimestamp) {
				const diff = BigInt(timeoutTimestamp) - BigInt(timestamp)
				await sleep(Number(diff))
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
		const self = this
		let request: RequestWithStatus | undefined
		while (!request) {
			await sleep(self.config.pollInterval)
			request = await self.queryRequest(hash)
		}

		let status =
			request.source === self.config.hyperbridge.stateMachineId
				? RequestStatus.HYPERBRIDGE_DELIVERED
				: RequestStatus.SOURCE
		const latestMetadata = request.statuses[request.statuses.length - 1]
		// start with the latest status
		status = maxBy(
			[status, latestMetadata.status as RequestStatus],
			(item) => REQUEST_STATUS_WEIGHTS[item as RequestStatus],
		)!

		while (true) {
			switch (status) {
				// request has been dispatched from source chain
				case RequestStatus.SOURCE: {
					let sourceUpdate: StateMachineUpdate | undefined
					while (!sourceUpdate) {
						await sleep(self.config.pollInterval)
						sourceUpdate = await self.queryStateMachineUpdateByHeight({
							statemachineId: request.source,
							height: request.statuses[0].metadata.blockNumber,
							chain: self.config.hyperbridge.stateMachineId,
						})
					}

					yield {
						status: RequestStatus.SOURCE_FINALIZED,
						metadata: {
							blockHash: sourceUpdate.blockHash,
							blockNumber: sourceUpdate.height,
							transactionHash: sourceUpdate.transactionHash,
						},
					}
					status = RequestStatus.SOURCE_FINALIZED
					break
				}

				// finality proofs for request has been verified on Hyperbridge
				case RequestStatus.SOURCE_FINALIZED: {
					// wait for the request to be delivered on Hyperbridge
					while (!request || request.statuses.length < 2) {
						await sleep(self.config.pollInterval)
						request = await self.queryRequest(hash)
					}

					status =
						request.dest === self.config.hyperbridge.stateMachineId
							? RequestStatus.DESTINATION
							: RequestStatus.HYPERBRIDGE_DELIVERED

					yield {
						status,
						metadata: {
							blockHash: request.statuses[1].metadata.blockHash,
							blockNumber: request.statuses[1].metadata.blockNumber,
							transactionHash: request.statuses[1].metadata.transactionHash,
						},
					}
					break
				}

				// the request has been verified and aggregated on Hyperbridge
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					// Get the latest state machine update for hyperbridge on the destination chain
					let hyperbridgeFinalized: StateMachineUpdate | undefined
					let index = request.source === self.config.hyperbridge.stateMachineId ? 0 : 1
					while (!hyperbridgeFinalized) {
						await sleep(self.config.pollInterval)
						hyperbridgeFinalized = await self.queryStateMachineUpdateByHeight({
							statemachineId: self.config.hyperbridge.stateMachineId,
							height: request.statuses[index].metadata.blockNumber,
							chain: request.dest,
						})
					}

					const destChain = await getChain(self.config.dest)
					const hyperbridge = await getChain({
						...self.config.hyperbridge,
						hasher: "Keccak",
					})

					const proof = await hyperbridge.queryRequestsProof(
						[postRequestCommitment(request)],
						request.dest,
						BigInt(hyperbridgeFinalized.height),
					)

					const calldata = destChain.encode({
						kind: "PostRequest",
						proof: {
							stateMachine: self.config.hyperbridge.stateMachineId,
							consensusStateId: self.config.hyperbridge.consensusStateId,
							proof,
							height: BigInt(hyperbridgeFinalized.height),
						},
						requests: [request],
						signer: pad("0x"),
					})

					yield {
						status: RequestStatus.HYPERBRIDGE_FINALIZED,
						metadata: {
							blockHash: hyperbridgeFinalized.blockHash,
							blockNumber: hyperbridgeFinalized.height,
							transactionHash: hyperbridgeFinalized.transactionHash,
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
						await sleep(self.config.pollInterval)
						request = await self.queryRequest(hash)
						delivered = request?.statuses.find((s) => s.status === RequestStatus.DESTINATION)
					}

					let index = request.source === self.config.hyperbridge.stateMachineId ? 1 : 2

					yield {
						status: RequestStatus.DESTINATION,
						metadata: {
							blockHash: request.statuses[index].metadata.blockHash,
							blockNumber: request.statuses[index].metadata.blockNumber,
							transactionHash: request.statuses[index].metadata.transactionHash,
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
		const self = this
		let request = await self.queryRequest(hash)
		if (!request) throw new Error(`Request not found`)

		const destChain = await getChain(self.config.dest)

		// if the destination is hyperbridge, then just wait for hyperbridge finality
		let status =
			request.dest === self.config.hyperbridge.stateMachineId
				? TimeoutStatus.HYPERBRIDGE_TIMED_OUT
				: TimeoutStatus.PENDING_TIMEOUT

		const commitment = postRequestCommitment(request)
		const hyperbridge = (await getChain({
			...self.config.hyperbridge,
			hasher: "Keccak",
		})) as unknown as SubstrateChain

		const latest = request.statuses[request.statuses.length - 1]

		// we're always interested in the latest status
		status = maxBy(
			[status, latest.status as TimeoutStatus],
			(item) => TIMEOUT_STATUS_WEIGHTS[item as TimeoutStatus],
		)!

		while (true) {
			switch (status) {
				case TimeoutStatus.PENDING_TIMEOUT: {
					const receipt = await hyperbridge.queryRequestReceipt(commitment)
					if (!receipt && request.source !== self.config.hyperbridge.stateMachineId) {
						status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
						break
					}

					let update: StateMachineUpdate | undefined
					while (!update) {
						await sleep(self.config.pollInterval)
						update = await self.queryStateMachineUpdateByTimestamp({
							statemachineId: request.dest,
							commitmentTimestamp: request.timeoutTimestamp,
							chain: self.config.hyperbridge.stateMachineId,
						})
					}

					yield {
						status: TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT,
						metadata: {
							blockHash: update.blockHash,
							blockNumber: update.height,
							transactionHash: update.transactionHash,
						},
					}
					status = TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT
					break
				}

				case TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT: {
					if (request.source !== self.config.hyperbridge.stateMachineId) {
						const receipt = await hyperbridge.queryRequestReceipt(commitment)
						if (!receipt) {
							status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
							break
						}
					}

					const update = (await self.queryStateMachineUpdateByTimestamp({
						statemachineId: request.dest,
						commitmentTimestamp: request.timeoutTimestamp,
						chain: self.config.hyperbridge.stateMachineId,
					}))!

					const proof = await destChain.queryStateProof(BigInt(update.height), [
						destChain.requestReceiptKey(commitment),
					])

					let { blockHash, transactionHash, blockNumber } = await hyperbridge.submitUnsigned({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: request.dest,
							consensusStateId: self.config.dest.consensusStateId,
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
						request.source === self.config.hyperbridge.stateMachineId
							? TimeoutStatus.TIMED_OUT
							: TimeoutStatus.HYPERBRIDGE_TIMED_OUT

					yield {
						status,
						metadata: {
							blockHash,
							transactionHash,
							blockNumber,
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
							await sleep(self.config.pollInterval)
							update = await self.queryStateMachineUpdateByTimestamp({
								statemachineId: self.config.hyperbridge.stateMachineId,
								commitmentTimestamp: request.timeoutTimestamp,
								chain: request.source,
							})
						}
					} else {
						let timeout: RequestStatusWithMetadata | undefined
						while (!timeout || timeout?.status !== TimeoutStatus.HYPERBRIDGE_TIMED_OUT) {
							await sleep(self.config.pollInterval)
							const req = await self.queryRequest(hash)
							if (!req) continue
							timeout = req.statuses
								.sort((a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status])
								.pop()
						}

						while (!update) {
							await sleep(self.config.pollInterval)
							update = await self.queryStateMachineUpdateByHeight({
								statemachineId: self.config.hyperbridge.stateMachineId,
								height: timeout.metadata.blockNumber,
								chain: request.source,
							})
						}
					}

					const proof = await hyperbridge.queryStateProof(BigInt(update.height), [
						hyperbridge.requestReceiptKey(commitment),
					])

					const sourceChain = await getChain(self.config.source)
					let calldata = sourceChain.encode({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: self.config.hyperbridge.stateMachineId,
							consensusStateId: self.config.hyperbridge.consensusStateId,
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
					yield {
						status: TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT,
						metadata: {
							transactionHash: update.transactionHash,
							blockNumber: update.blockNumber,
							blockHash: update.blockHash,
							calldata,
						},
					}
					status = TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT
					break
				}

				case TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT:
				case TimeoutStatus.TIMED_OUT:
					return
			}
		}
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
	private async withRetry<T>(
		operation: () => Promise<T>,
		retryConfig: RetryConfig = this.defaultRetryConfig,
	): Promise<T> {
		let lastError
		for (let i = 0; i < retryConfig.maxRetries; i++) {
			try {
				return await operation()
			} catch (error) {
				lastError = error
				await new Promise((resolve) => setTimeout(resolve, retryConfig.backoffMs * Math.pow(2, i)))
			}
		}
		throw lastError
	}
}
