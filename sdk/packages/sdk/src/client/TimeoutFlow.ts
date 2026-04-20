import { maxBy } from "lodash-es"
import { pad } from "viem"

import type { IChain, SubstrateChain } from "@/chain"
import {
	type HexString,
	type PostRequestTimeoutStatus,
	type PostRequestWithStatus,
	type RequestStatusWithMetadata,
	type StateMachineUpdate,
	type TimeoutStatusKey,
	RequestStatus,
	TimeoutStatus,
} from "@/types"
import {
	COMBINED_STATUS_WEIGHTS,
	TIMEOUT_STATUS_WEIGHTS,
	parseStateMachineId,
	postRequestCommitment,
	waitForChallengePeriod,
} from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { StateMachineQueries } from "./StateMachineQueries"
import type { ClientContext } from "./types"
import { sleepFor, waitOrAbort } from "./utils"

/**
 * Timeout flow: pending / finalized / timed-out status handling for POST requests
 * that have passed their `timeoutTimestamp`. Also hosts the shared `timeoutStream`
 * watcher (used by both {@link PostRequestStatus} and {@link GetRequestStatus})
 * and the `aggregateTransactionWithCommitment` helper.
 */
export class TimeoutFlow {
	private readonly logger

	constructor(
		private readonly ctx: ClientContext,
		private readonly queries: StateMachineQueries,
	) {
		this.logger = ctx.logger.withTag("[TimeoutFlow]")
	}

	/**
	 * Watcher that yields exactly one `PENDING_TIMEOUT` event when the chain's
	 * current timestamp passes `timeoutTimestamp`. Yields nothing if no timeout.
	 */
	async *timeoutStream(
		timeoutTimestamp: bigint,
		chain: IChain,
	): AsyncGenerator<RequestStatusWithMetadata, void> {
		const logger = this.logger.withTag("[timeoutStream()]")
		if (timeoutTimestamp === 0n) return

		let timestamp = await chain.timestamp()
		while (timestamp < timeoutTimestamp) {
			logger.trace("Comparing timeout timestamps", { control: timeoutTimestamp, latest: timestamp })
			const diff = BigInt(timeoutTimestamp) - BigInt(timestamp)
			await sleepFor(this.ctx, Number(diff))
			timestamp = await chain.timestamp()
		}

		yield {
			status: TimeoutStatus.PENDING_TIMEOUT,
			metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
		}
	}

	/**
	 * Populates timeout-related status events (`PENDING_TIMEOUT`,
	 * `DESTINATION_FINALIZED_TIMEOUT`, `HYPERBRIDGE_FINALIZED_TIMEOUT`) and the
	 * accompanying timeout-proof calldata.
	 */
	async addTimeoutFinalityEvents(request: PostRequestWithStatus): Promise<PostRequestWithStatus> {
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const events: RequestStatusWithMetadata[] = []
		const commitment = postRequestCommitment(request).commitment
		const receipt = await destChain.queryRequestReceipt(commitment)
		const destTimestamp = await destChain.timestamp()

		const commit = (req: PostRequestWithStatus) => {
			this.logger.trace(`Added ${events.length} timeout events`, events)
			request.statuses = [...req.statuses, ...events]
			return request
		}

		if (request.timeoutTimestamp === 0n) return commit(request)
		if (receipt || request.timeoutTimestamp > destTimestamp) return commit(request)

		const is_finished = request.statuses.find((item) => item.status === RequestStatus.DESTINATION)
		if (!is_finished) {
			events.push({
				status: TimeoutStatus.PENDING_TIMEOUT,
				metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
			})
		}

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
			const stream = this.streamInternal(hash, controller.signal)
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

	/**
	 * Aggregate a relay-dispatched transaction's request through Hyperbridge, returning
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

	private async *streamInternal(
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
}
