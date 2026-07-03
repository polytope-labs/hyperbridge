import { maxBy } from "lodash-es"
import { pad } from "viem"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import type { SubstrateChain } from "@/chain"
import { EvmChain } from "@/chains/evm"
import {
	type HexString,
	type PostRequestTimeoutStatus,
	type PostRequestWithStatus,
	type RequestStatusKey,
	type RequestStatusWithMetadata,
	type StateMachineUpdate,
	type TimeoutStatusKey,
	RequestStatus,
	TimeoutStatus,
} from "@/types"
import {
	COMBINED_STATUS_WEIGHTS,
	REQUEST_STATUS_WEIGHTS,
	TIMEOUT_STATUS_WEIGHTS,
	parseStateMachineId,
	postRequestCommitment,
	waitForChallengePeriod,
} from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { Queries } from "./Queries"
import type { ClientContext } from "."
import { timeoutStream, waitOrAbort, withRetry } from "./utils"

/**
 * POST request lifecycle — snapshot status, streaming status, and the full
 * timeout flow (pending → destination-finalized → hyperbridge-finalized → timed-out).
 * Also hosts `aggregateTransactionWithCommitment`, which relays a dispatched
 * post request through Hyperbridge from source-chain state.
 */
export class PostRequestClient {
	private readonly logger

	constructor(
		private readonly ctx: ClientContext,
		private readonly queries: Queries,
	) {
		this.logger = ctx.logger.withTag("[PostRequestClient]")
	}

	/**
	 * Enhances a request with finality events by querying state machine updates.
	 * Adds `SOURCE_FINALIZED` and `HYPERBRIDGE_FINALIZED` when applicable.
	 */
	async addRequestFinalityEvents(request: PostRequestWithStatus): Promise<PostRequestWithStatus> {
		const events: RequestStatusWithMetadata[] = []

		const commit = () => {
			this.logger.trace(`Added ${events.length} 'Request' finality events`, events)
			request.statuses = [...request.statuses, ...events]
			return request
		}

		let hyperbridgeDelivered: RequestStatusWithMetadata | undefined

		if (request.source === this.ctx.config.hyperbridge.config.stateMachineId) {
			hyperbridgeDelivered = request.statuses[0]
		} else {
			const sourceFinality = await this.queries.queryStateMachineUpdateByHeight({
				statemachineId: request.source,
				height: request.statuses[0].metadata.blockNumber,
				chain: this.ctx.config.hyperbridge.config.stateMachineId,
			})
			if (!sourceFinality) return commit()

			events.push({
				status: RequestStatus.SOURCE_FINALIZED,
				metadata: {
					blockHash: sourceFinality.blockHash,
					blockNumber: sourceFinality.height,
					transactionHash: sourceFinality.transactionHash,
					timestamp: sourceFinality.timestamp,
				},
			})

			hyperbridgeDelivered = request.statuses.find((item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED)
			if (!hyperbridgeDelivered) return commit()
		}

		if (request.statuses.some((s) => s.status === TimeoutStatus.HYPERBRIDGE_TIMED_OUT)) return commit()
		if (request.dest === this.ctx.config.hyperbridge.config.stateMachineId) return commit()

		const finalized = await this.buildFinalized(request, hyperbridgeDelivered)
		if (!finalized) return commit()

		events.push(finalized)
		return commit()
	}

	/**
	 * Snapshot query: request + all inferred finality/timeout events, sorted.
	 */
	async queryRequestWithStatus(hash: HexString): Promise<PostRequestWithStatus | undefined> {
		let request = await this.queries.queryPostRequest(hash)
		if (!request) return

		request = await this.addRequestFinalityEvents(request)
		request = await this.addTimeoutFinalityEvents(request)

		request.statuses = request.statuses.sort(
			(a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status],
		)
		return request
	}

	/**
	 * Streaming status updates for a post request. Ends when the request reaches
	 * the destination or a timeout becomes pending.
	 */
	async *postRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		const controller = new AbortController()
		const logger = this.logger.withTag("[postRequestStatusStream]")

		try {
			const request = await waitOrAbort(this.ctx, {
				signal: controller.signal,
				promise: () => this.queries.queryPostRequest(hash),
			})

			logger.trace("`Request` found")
			const chain = this.ctx.config.dest
			const timeouts =
				request.timeoutTimestamp > 0n ? timeoutStream(this.ctx, request.timeoutTimestamp, chain) : undefined
			const statusStream = this.streamStatusInternal(hash, controller.signal)
			const combined = timeouts ? mergeRace(timeouts, statusStream) : statusStream

			let item = await combined.next()
			while (!item.done) {
				logger.trace(`Yielding Event(${item.value.status})`)
				yield item.value
				if (
					item.value.status === RequestStatus.DESTINATION ||
					item.value.status === TimeoutStatus.PENDING_TIMEOUT
				)
					break
				item = await combined.next()
			}
			logger.trace("Streaming complete")
		} catch (error) {
			if (!AbortSignalInternal.isError(error)) throw error
		}
		controller.abort()
	}

	private async *streamStatusInternal(
		hash: HexString,
		signal: AbortSignal,
	): AsyncGenerator<RequestStatusWithMetadata, void> {
		let request = await waitOrAbort(this.ctx, { signal, promise: () => this.queries.queryPostRequest(hash) })

		let status: RequestStatusKey =
			request.source === this.ctx.config.hyperbridge.config.stateMachineId
				? RequestStatus.HYPERBRIDGE_DELIVERED
				: RequestStatus.SOURCE

		const latestMetadata = request.statuses[request.statuses.length - 1]
		const latest_request = maxBy(
			[status, latestMetadata.status as RequestStatusKey],
			(item) => REQUEST_STATUS_WEIGHTS[item],
		)
		if (!latest_request) return
		status = latest_request

		while (true) {
			switch (status) {
				case RequestStatus.SOURCE: {
					const sourceUpdate = await waitOrAbort(this.ctx, {
						signal,
						promise: () =>
							this.queries.queryStateMachineUpdateByHeight({
								statemachineId: request.source,
								height: request.statuses[0].metadata.blockNumber,
								chain: this.ctx.config.hyperbridge.config.stateMachineId,
							}),
					})

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

				case RequestStatus.SOURCE_FINALIZED: {
					request = await waitOrAbort(this.ctx, {
						signal,
						promise: () => this.queries.queryPostRequest(hash),
						predicate: (r) => !r || r.statuses.length < 2,
					})

					status =
						request.dest === this.ctx.config.hyperbridge.config.stateMachineId
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

				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					const stateMachineId = this.ctx.config.hyperbridge.config.stateMachineId
					const index = request.source === stateMachineId ? 0 : 1

					yield await this.streamFinalized(signal, request, index)
					status = RequestStatus.HYPERBRIDGE_FINALIZED
					break
				}

				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					request = await waitOrAbort(this.ctx, {
						signal,
						promise: () => this.queries.queryPostRequest(hash),
						predicate: (r) => !r || !r.statuses.find((s) => s.status === RequestStatus.DESTINATION),
					})

					const index = request.source === this.ctx.config.hyperbridge.config.stateMachineId ? 1 : 2
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

	/**
	 * Populates timeout-related status events (`PENDING_TIMEOUT`,
	 * `DESTINATION_FINALIZED_TIMEOUT`, `HYPERBRIDGE_FINALIZED_TIMEOUT`) and the
	 * accompanying timeout-proof calldata.
	 */
	async addTimeoutFinalityEvents(request: PostRequestWithStatus): Promise<PostRequestWithStatus> {
		const events: RequestStatusWithMetadata[] = []

		const commit = (req: PostRequestWithStatus) => {
			this.logger.trace(`Added ${events.length} timeout events`, events)
			request.statuses = [...req.statuses, ...events]
			return request
		}

		if (request.timeoutTimestamp === 0n) return commit(request)
		// Skip timeout inference once the request has reached a terminal status.
		if (
			request.statuses.some(
				(item) => item.status === RequestStatus.DESTINATION || item.status === TimeoutStatus.TIMED_OUT,
			)
		)
			return commit(request)

		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const commitment = postRequestCommitment(request).commitment
		const receipt = await destChain.queryRequestReceipt(commitment)
		const destTimestamp = await destChain.timestamp()

		if (receipt || request.timeoutTimestamp > destTimestamp) return commit(request)

		events.push({
			status: TimeoutStatus.PENDING_TIMEOUT,
			metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
		})

		const delivered = request.statuses.find((item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED)

		let hyperbridgeFinalized: StateMachineUpdate | undefined
		if (!delivered) {
			hyperbridgeFinalized = await this.queries.queryStateMachineUpdateByTimestamp({
				statemachineId: this.ctx.config.hyperbridge.config.stateMachineId,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: request.source,
			})
		} else {
			const destFinalized = await this.queries.queryStateMachineUpdateByTimestamp({
				statemachineId: request.dest,
				commitmentTimestamp: request.timeoutTimestamp,
				chain: this.ctx.config.hyperbridge.config.stateMachineId,
			})
			if (!destFinalized) return commit(request)

			events.push({
				status: TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT,
				metadata: {
					blockHash: destFinalized.blockHash,
					blockNumber: destFinalized.blockNumber,
					transactionHash: destFinalized.transactionHash,
					timestamp: destFinalized.timestamp,
				},
			})

			if (request.source === this.ctx.config.hyperbridge.config.stateMachineId) return request

			const hyperbridgeTimedOut = request.statuses.find(
				(item) => item.status === TimeoutStatus.HYPERBRIDGE_TIMED_OUT,
			)
			if (!hyperbridgeTimedOut) return commit(request)
			hyperbridgeFinalized = await this.queries.queryStateMachineUpdateByHeight({
				statemachineId: this.ctx.config.hyperbridge.config.stateMachineId,
				height: hyperbridgeTimedOut.metadata.blockNumber,
				chain: request.source,
			})
		}

		if (!hyperbridgeFinalized) return commit(request)

		const proof = await hyperbridge.queryStateProof(BigInt(hyperbridgeFinalized.height), [
			hyperbridge.requestReceiptKey(commitment),
		])
		const sourceChain = this.ctx.config.source
		const calldata = sourceChain.encode({
			kind: "TimeoutPostRequest",
			proof: {
				proof,
				height: BigInt(hyperbridgeFinalized.height),
				stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
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

		return commit(request)
	}

	/**
	 * Streaming status updates for a timed-out post request.
	 */
	async *postRequestTimeoutStream(hash: HexString): AsyncGenerator<PostRequestTimeoutStatus, void> {
		const controller = new AbortController()
		const logger = this.logger.withTag("[postRequestTimeoutStream]")

		try {
			const request = await this.queries.queryPostRequest(hash)
			if (!request) throw new Error("Request not found")

			logger.trace("`Request` found")
			const stream = this.streamTimeoutInternal(hash, controller.signal)
			let item = await stream.next()
			while (!item.done) {
				logger.trace(`Yielding Timeout Event(${item.value.status})`)
				yield item.value
				item = await stream.next()
			}
			logger.trace("Streaming complete")
		} catch (error) {
			if (!AbortSignalInternal.isError(error)) throw error
		}
		controller.abort()
	}

	private async *streamTimeoutInternal(
		hash: HexString,
		signal: AbortSignal,
	): AsyncGenerator<PostRequestTimeoutStatus, void> {
		const request = await waitOrAbort(this.ctx, { signal, promise: () => this.queries.queryPostRequest(hash) })

		const destChain = this.ctx.config.dest
		let status: TimeoutStatusKey =
			request.dest === this.ctx.config.hyperbridge.config.stateMachineId
				? TimeoutStatus.HYPERBRIDGE_TIMED_OUT
				: TimeoutStatus.PENDING_TIMEOUT

		const commitment = postRequestCommitment(request).commitment
		const hyperbridge = this.ctx.config.hyperbridge as SubstrateChain

		const latest = request.statuses[request.statuses.length - 1]
		const latest_request = maxBy(
			[status, latest.status as TimeoutStatusKey],
			(item: TimeoutStatusKey) => TIMEOUT_STATUS_WEIGHTS[item],
		)
		if (!latest_request) return
		status = latest_request

		while (true) {
			switch (status) {
				case TimeoutStatus.PENDING_TIMEOUT: {
					const receipt = await hyperbridge.queryRequestReceipt(commitment)
					if (!receipt && request.source !== this.ctx.config.hyperbridge.config.stateMachineId) {
						status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
						break
					}

					const update = await waitOrAbort(this.ctx, {
						signal,
						promise: () =>
							this.queries.queryStateMachineUpdateByTimestamp({
								statemachineId: request.dest,
								commitmentTimestamp: request.timeoutTimestamp,
								chain: this.ctx.config.hyperbridge.config.stateMachineId,
							}),
					})

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
					if (request.source !== this.ctx.config.hyperbridge.config.stateMachineId) {
						const receipt = await hyperbridge.queryRequestReceipt(commitment)
						if (!receipt) {
							status = TimeoutStatus.HYPERBRIDGE_TIMED_OUT
							break
						}
					}

					const update = (await this.queries.queryStateMachineUpdateByTimestamp({
						statemachineId: request.dest,
						commitmentTimestamp: request.timeoutTimestamp,
						chain: this.ctx.config.hyperbridge.config.stateMachineId,
					}))!

					const proof = await destChain.queryStateProof(BigInt(update.height), [
						destChain.requestReceiptKey(commitment),
					])

					const { stateId } = parseStateMachineId(request.dest)
					await waitForChallengePeriod(hyperbridge, {
						height: BigInt(update.height),
						id: { stateId, consensusStateId: this.ctx.config.dest.config.consensusStateId },
					})

					const { blockHash, transactionHash, blockNumber, timestamp } = await hyperbridge.submitUnsigned({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: request.dest,
							consensusStateId: this.ctx.config.dest.config.consensusStateId,
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
						request.source === this.ctx.config.hyperbridge.config.stateMachineId
							? TimeoutStatus.TIMED_OUT
							: TimeoutStatus.HYPERBRIDGE_TIMED_OUT

					yield { status, metadata: { blockHash, transactionHash, blockNumber, timestamp } }
					break
				}

				case TimeoutStatus.HYPERBRIDGE_TIMED_OUT: {
					const hasDelivered = request.statuses.some(
						(item) => item.status === RequestStatus.HYPERBRIDGE_DELIVERED,
					)
					let update: StateMachineUpdate | undefined
					if (!hasDelivered) {
						update = await waitOrAbort(this.ctx, {
							signal,
							promise: () =>
								this.queries.queryStateMachineUpdateByTimestamp({
									statemachineId: this.ctx.config.hyperbridge.config.stateMachineId,
									commitmentTimestamp: request.timeoutTimestamp,
									chain: request.source,
								}),
						})
					} else {
						const timeout = await waitOrAbort(this.ctx, {
							signal,
							promise: async () => {
								const req = await this.queries.queryPostRequest(hash)
								return req?.statuses
									.sort(
										(a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status],
									)
									.pop()
							},
							predicate: (t) => !t || t?.status !== TimeoutStatus.HYPERBRIDGE_TIMED_OUT,
						})

						update = await waitOrAbort(this.ctx, {
							signal,
							promise: async () =>
								this.queries.queryStateMachineUpdateByHeight({
									statemachineId: this.ctx.config.hyperbridge.config.stateMachineId,
									height: timeout.metadata.blockNumber,
									chain: request.source,
								}),
						})
					}

					const proof = await hyperbridge.queryStateProof(BigInt(update.height), [
						hyperbridge.requestReceiptKey(commitment),
					])

					const sourceChain = this.ctx.config.source
					const calldata = sourceChain.encode({
						kind: "TimeoutPostRequest",
						proof: {
							proof,
							height: BigInt(update.height),
							stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
							consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
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

					const { stateId } = parseStateMachineId(this.ctx.config.hyperbridge.config.stateMachineId)
					await waitForChallengePeriod(sourceChain, {
						height: BigInt(update.height),
						id: { stateId, consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId },
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
					const delivered = await waitOrAbort(this.ctx, {
						signal,
						promise: async () => {
							const req = await this.queries.queryPostRequest(hash)
							return req?.statuses.find((s) => s.status === RequestStatus.TIMED_OUT)
						},
					})
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
	 * Snapshot helper: returns the `HYPERBRIDGE_FINALIZED` event with relayer
	 * calldata if prerequisites are met, or `undefined` if we're still waiting
	 * for a consensus proof.
	 */
	private async buildFinalized(
		request: PostRequestWithStatus,
		hyperbridgeDelivered: RequestStatusWithMetadata,
	): Promise<RequestStatusWithMetadata | undefined> {
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const { config } = hyperbridge

		// Check for existing finality first. Hyperbridge runs a light client of itself
		// (pallet-beefy-consensus-proofs), so if its self state-machine update already finalizes the
		// message, deliver with a plain PostRequest proof — no consensus proof needed.
		const finality = await this.queries.queryStateMachineUpdateByHeight({
			statemachineId: config.stateMachineId,
			height: hyperbridgeDelivered.metadata.blockNumber,
			chain: request.dest,
		})
		if (finality) {
			const proof = await hyperbridge.queryProof(
				{ Requests: [postRequestCommitment(request).commitment] },
				request.dest,
				BigInt(finality.height),
			)
			const calldata = destChain.encode({
				kind: "PostRequest",
				proof: {
					stateMachine: config.stateMachineId,
					consensusStateId: config.consensusStateId,
					proof,
					height: BigInt(finality.height),
				},
				requests: [request],
				signer: pad("0x"),
			})
			return {
				status: RequestStatus.HYPERBRIDGE_FINALIZED,
				metadata: {
					blockHash: finality.blockHash,
					blockNumber: finality.height,
					transactionHash: finality.transactionHash,
					timestamp: finality.timestamp,
					calldata,
				},
			}
		}

		// No existing finality. For an EVM destination, advance its Hyperbridge light client and
		// deliver in one batch (consensus proof + message proof).
		if (destChain instanceof EvmChain) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const currentEpoch = await destChain.currentEpoch()
			const consensusResult = await hyperbridgeSubstrate.queryConsensusProofs(
				BigInt(hyperbridgeDelivered.metadata.blockNumber),
				currentEpoch,
			)
			if (!consensusResult) return undefined

			const proof = await hyperbridge.queryProof(
				{ Requests: [postRequestCommitment(request).commitment] },
				request.dest,
				consensusResult.provenHeight,
			)

			const calldata = destChain.encode({
				kind: "BatchConsensusAndPostRequest",
				consensusProofs: consensusResult.proofs,
				proof: {
					stateMachine: config.stateMachineId,
					consensusStateId: config.consensusStateId,
					proof,
					height: consensusResult.provenHeight,
				},
				requests: [request],
				signer: pad("0x"),
			})

			return {
				status: RequestStatus.HYPERBRIDGE_FINALIZED,
				metadata: {
					blockHash: hyperbridgeDelivered.metadata.blockHash,
					blockNumber: Number(consensusResult.provenHeight),
					transactionHash: hyperbridgeDelivered.metadata.transactionHash,
					// @ts-ignore
					timestamp: hyperbridgeDelivered.metadata.timestamp,
					calldata,
				},
			}
		}

		return undefined
	}

	/**
	 * Streaming helper: waits for the consensus proof, fetches the messaging
	 * proof with retry, and returns the finalized event. Caller has already
	 * observed `HYPERBRIDGE_DELIVERED` and provides the index into
	 * `request.statuses` that carries it.
	 */
	private async streamFinalized(
		signal: AbortSignal,
		request: PostRequestWithStatus,
		hyperbridgeDeliveredIndex: number,
	): Promise<RequestStatusWithMetadata> {
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const stateMachineId = this.ctx.config.hyperbridge.config.stateMachineId
		const neededHeight = BigInt(request.statuses[hyperbridgeDeliveredIndex].metadata.blockNumber)

		this.logger.trace(`[streamFinalized] neededHeight=${neededHeight}`)

		const commitment = postRequestCommitment(request).commitment

		// Check for existing finality first (Hyperbridge's self state-machine update).
		let finality = await this.queries.queryStateMachineUpdateByHeight({
			statemachineId: stateMachineId,
			height: Number(neededHeight),
			chain: request.dest,
		})

		// No existing finality and the destination is EVM: advance its Hyperbridge light client and
		// deliver in one batch (consensus proof + message proof).
		if (!finality && destChain instanceof EvmChain) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const currentEpoch = await destChain.currentEpoch()
			const consensusResult = await waitOrAbort(this.ctx, {
				signal,
				promise: () => hyperbridgeSubstrate.queryConsensusProofs(neededHeight, currentEpoch),
			})

			this.logger.trace(
				`[streamFinalized] consensusProofs found (${consensusResult.proofs.length} proofs), provenHeight=${consensusResult.provenHeight}, ` +
					`commitment=${commitment}, dest=${request.dest}`,
			)

			const proof = await this.fetchProofWithRetry(signal, () =>
				hyperbridge.queryProof({ Requests: [commitment] }, request.dest, consensusResult.provenHeight),
			)

			const calldata = destChain.encode({
				kind: "BatchConsensusAndPostRequest",
				consensusProofs: consensusResult.proofs,
				proof: {
					stateMachine: stateMachineId,
					consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
					proof,
					height: consensusResult.provenHeight,
				},
				requests: [request],
				signer: pad("0x"),
			})

			return {
				status: RequestStatus.HYPERBRIDGE_FINALIZED,
				metadata: {
					blockHash: request.statuses[hyperbridgeDeliveredIndex].metadata.blockHash,
					blockNumber: Number(consensusResult.provenHeight),
					transactionHash: request.statuses[hyperbridgeDeliveredIndex].metadata.transactionHash,
					// @ts-ignore
					timestamp: request.statuses[hyperbridgeDeliveredIndex].metadata.timestamp,
					calldata,
				},
			}
		}

		// Otherwise wait for Hyperbridge's
		// finality on dest chain then deliver with a plain PostRequest proof.
		if (!finality) {
			finality = await waitOrAbort(this.ctx, {
				signal,
				promise: () =>
					this.queries.queryStateMachineUpdateByHeight({
						statemachineId: stateMachineId,
						height: Number(neededHeight),
						chain: request.dest,
					}),
			})
		}

		const proof = await this.fetchProofWithRetry(signal, () =>
			hyperbridge.queryProof({ Requests: [commitment] }, request.dest, BigInt(finality.height)),
		)

		// Substrate destinations must wait out the consensus challenge period before the proof is usable.
		if (!(destChain instanceof EvmChain)) {
			const { stateId } = parseStateMachineId(stateMachineId)
			await waitForChallengePeriod(destChain, {
				height: BigInt(finality.height),
				id: { stateId, consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId },
			})
		}

		const calldata = destChain.encode({
			kind: "PostRequest",
			proof: {
				stateMachine: stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
				proof,
				height: BigInt(finality.height),
			},
			requests: [request],
			signer: pad("0x"),
		})

		return {
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: finality.blockHash,
				blockNumber: finality.height,
				transactionHash: finality.transactionHash,
				timestamp: finality.timestamp,
				calldata,
			},
		}
	}

	/**
	 * Retry-wrapped `hyperbridge.queryProof`. Fails after 6 retries (~2 minutes at
	 * 2s backoff) so a hung Hyperbridge node doesn't stall the stream forever.
	 */
	private async fetchProofWithRetry(signal: AbortSignal, fetch: () => Promise<HexString>): Promise<HexString> {
		let attempt = 0
		const safe = async () => {
			attempt++
			try {
				this.logger.trace(`[fetchProofWithRetry] attempt ${attempt}`)
				const result = await fetch()
				this.logger.trace(`[fetchProofWithRetry] attempt ${attempt} succeeded, proof length=${result.length}`)
				return { data: result, error: null as unknown }
			} catch (err) {
				this.logger.trace(`[fetchProofWithRetry] attempt ${attempt} failed: ${err}`)
				return { data: null, error: err as unknown }
			}
		}
		const result = await waitOrAbort(this.ctx, {
			signal,
			promise: () => withRetry(this.ctx, safe, { backoffMs: 2000, maxRetries: 6 }),
		})
		if (result.data === null) {
			this.logger.error("Failed to fetch messaging proof:", result.error)
			throw result.error
		}
		return result.data
	}

	/**
	 * Relay a post-request to Hyperbridge from source-chain state, returning
	 * the finalized Hyperbridge extrinsic receipt.
	 */
	async aggregateTransactionWithCommitment(
		commitment: HexString,
	): Promise<Awaited<ReturnType<SubstrateChain["submitUnsigned"]>>> {
		const logger = this.logger.withTag("aggregateTransactionWithCommitment")

		const { stateMachineId, consensusStateId } = this.ctx.config.source.config
		const sourceChain = this.ctx.config.source
		const hyperbridge = this.ctx.config.hyperbridge as SubstrateChain

		logger.trace("Querying post request with commitment hash")
		const request = await this.queries.queryPostRequest(commitment)
		if (!request) throw new Error("Request not found")

		logger.trace("Fetch latest stateMachineHeight")
		const latestStateMachineHeight = await hyperbridge.latestStateMachineHeight({
			stateId: parseStateMachineId(stateMachineId).stateId,
			consensusStateId,
		})

		logger.trace("Query Request Proof from sourceChain")
		const proof = await sourceChain.queryProof(
			{ Requests: [commitment] },
			this.ctx.config.hyperbridge.config.stateMachineId,
			latestStateMachineHeight,
		)

		logger.trace("Construct Extrinsic and Submit Unsigned")
		return hyperbridge.submitUnsigned({
			kind: "PostRequest",
			proof: {
				stateMachine: this.ctx.config.source.config.stateMachineId,
				consensusStateId: this.ctx.config.source.config.consensusStateId,
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
	}
}
