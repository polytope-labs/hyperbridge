import { maxBy } from "lodash-es"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import type { IChain } from "@/chain"
import {
	type HexString,
	type PostRequestWithStatus,
	type RequestStatusKey,
	type RequestStatusWithMetadata,
	RequestStatus,
	TimeoutStatus,
} from "@/types"
import { COMBINED_STATUS_WEIGHTS, REQUEST_STATUS_WEIGHTS } from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { ProofFinalizer } from "./ProofFinalizer"
import type { StateMachineQueries } from "./StateMachineQueries"
import type { ClientContext } from "./types"
import { waitOrAbort } from "./utils"

/**
 * POST request status tracking — snapshot (`queryRequestWithStatus`) and
 * streaming (`postRequestStatusStream`) flows. Finality events and stream
 * transitions are shared with {@link TimeoutFlow} (which fills in timeout
 * events) and {@link ProofFinalizer} (which assembles HYPERBRIDGE_FINALIZED).
 */
export class PostRequestStatus {
	private readonly logger

	constructor(
		private readonly ctx: ClientContext,
		private readonly queries: StateMachineQueries,
		private readonly proofFinalizer: ProofFinalizer,
		private readonly timeoutStream: (
			timeoutTimestamp: bigint,
			chain: IChain,
		) => AsyncGenerator<RequestStatusWithMetadata, void>,
		private readonly addTimeoutFinalityEvents: (
			request: PostRequestWithStatus,
		) => Promise<PostRequestWithStatus>,
	) {
		this.logger = ctx.logger.withTag("[PostRequestStatus]")
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

		const finalized = await this.proofFinalizer.buildPostRequestFinalized(request, hyperbridgeDelivered)
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
			const timeoutStream =
				request.timeoutTimestamp > 0n ? this.timeoutStream(request.timeoutTimestamp, chain) : undefined
			const statusStream = this.streamInternal(hash, controller.signal)

			logger.trace("Listening for events")
			const combined = timeoutStream ? mergeRace(timeoutStream, statusStream) : statusStream

			let item = await combined.next()
			while (!item.done) {
				logger.trace(`Yielding Event(${item.value.status})`)
				yield item.value
				item = await combined.next()
			}
			logger.trace("Streaming complete")
		} catch (error) {
			if (!AbortSignalInternal.isError(error)) throw error
		}
		controller.abort()
	}

	private async *streamInternal(
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

					yield await this.proofFinalizer.streamPostRequestFinalized(signal, request, index)
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
}
