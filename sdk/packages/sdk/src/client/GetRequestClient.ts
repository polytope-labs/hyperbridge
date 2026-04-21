import { maxBy } from "lodash-es"
import { pad } from "viem"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import type { SubstrateChain } from "@/chain"
import { EvmChain } from "@/chains/evm"
import {
	type GetRequestWithStatus,
	type HexString,
	type RequestStatusKey,
	type RequestStatusWithMetadata,
	type ResponseCommitmentWithValues,
	RequestStatus,
} from "@/types"
import { COMBINED_STATUS_WEIGHTS, REQUEST_STATUS_WEIGHTS } from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { Queries } from "./Queries"
import type { ClientContext } from "."
import { timeoutStream, waitOrAbort } from "./utils"

/**
 * GET request status tracking — snapshot + streaming flows. Responses travel
 * back to the origin chain, so finality work happens against `source` rather
 * than `dest`.
 */
export class GetRequestClient {
	private readonly logger

	constructor(
		private readonly ctx: ClientContext,
		private readonly queries: Queries,
	) {
		this.logger = ctx.logger.withTag("[GetRequestClient]")
	}

	/**
	 * Enhances a GET request with finality events (`SOURCE_FINALIZED`,
	 * `HYPERBRIDGE_FINALIZED`).
	 */
	async addGetRequestFinalityEvents(request: GetRequestWithStatus): Promise<GetRequestWithStatus> {
		const events: RequestStatusWithMetadata[] = []

		const commit = () => {
			this.logger.trace(`Added ${events.length} 'GetRequest' finality events`, events)
			request.statuses = [...request.statuses, ...events]
			return request
		}

		let hyperbridgeDelivered: RequestStatusWithMetadata | undefined

		if (request.source === this.ctx.config.hyperbridge.config.stateMachineId) {
			hyperbridgeDelivered = request.statuses[0]
			return commit()
		}

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

		try {
			const response = await this.queries.queryResponseByRequestId(request.commitment)
			if (!response) return commit()

			const finalized = await this.buildFinalized(request, hyperbridgeDelivered, response)
			if (finalized) events.push(finalized)
		} catch (error) {
			this.logger.trace("Could not generate HYPERBRIDGE_FINALIZED event for GET request:", error)
		}

		return commit()
	}

	/**
	 * Snapshot query with all inferred finality events sorted.
	 */
	async queryGetRequestWithStatus(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		let request = await this.queries.queryGetRequest(hash)
		if (!request) return

		request = await this.addGetRequestFinalityEvents(request)

		request.statuses = request.statuses.sort(
			(a, b) => COMBINED_STATUS_WEIGHTS[a.status] - COMBINED_STATUS_WEIGHTS[b.status],
		)
		return request
	}

	/**
	 * Streaming status updates for a GET request. Ends when the request reaches
	 * the destination or a timeout becomes pending.
	 */
	async *getRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		const controller = new AbortController()
		try {
			const request = await waitOrAbort(this.ctx, {
				signal: controller.signal,
				promise: () => this.queries.queryGetRequest(hash),
			})

			const chain = this.ctx.config.dest
			const timeouts =
				request.timeoutTimestamp > 0n ? timeoutStream(this.ctx, request.timeoutTimestamp, chain) : undefined
			const statusStream = this.streamInternal(hash, controller.signal)
			const combined = timeouts ? mergeRace(timeouts, statusStream) : statusStream

			let item = await combined.next()
			while (!item.done) {
				yield item.value
				item = await combined.next()
			}
		} catch (error) {
			if (!AbortSignalInternal.isError(error)) throw error
		}
		controller.abort()
	}

	private async *streamInternal(
		hash: HexString,
		signal: AbortSignal,
	): AsyncGenerator<RequestStatusWithMetadata, void> {
		let request = await waitOrAbort(this.ctx, { signal, promise: () => this.queries.queryGetRequest(hash) })

		let status: RequestStatusKey | undefined =
			request.source === this.ctx.config.hyperbridge.config.stateMachineId
				? RequestStatus.HYPERBRIDGE_DELIVERED
				: RequestStatus.SOURCE

		const latestMetadata = request.statuses[request.statuses.length - 1]
		status = maxBy([status, latestMetadata.status as RequestStatusKey], (item) => REQUEST_STATUS_WEIGHTS[item])
		if (!status) return

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
						promise: () => this.queries.queryGetRequest(hash),
						predicate: (r) => !r || r.statuses.length < 2,
					})

					status =
						request.source === this.ctx.config.hyperbridge.config.stateMachineId
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
					if (request.source === this.ctx.config.hyperbridge.config.stateMachineId) return

					const response = await this.queries.queryResponseByRequestId(hash)
					yield await this.streamFinalized(signal, request, 1, response)
					status = RequestStatus.HYPERBRIDGE_FINALIZED
					break
				}

				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					if (request.source === this.ctx.config.hyperbridge.config.stateMachineId) return

					request = await waitOrAbort(this.ctx, {
						signal,
						promise: () => this.queries.queryGetRequest(hash),
						predicate: (r) => !r || !r.statuses.find((s) => s.status === RequestStatus.DESTINATION),
					})

					yield {
						status: RequestStatus.DESTINATION,
						metadata: {
							blockHash: request.statuses[2].metadata.blockHash,
							blockNumber: request.statuses[2].metadata.blockNumber,
							transactionHash: request.statuses[2].metadata.transactionHash,
							// @ts-ignore
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
	 * Snapshot helper: returns the `HYPERBRIDGE_FINALIZED` event with source-chain
	 * calldata if prerequisites are met, or `undefined` if we're still waiting
	 * for a consensus proof (HandlerV2) or state machine update (HandlerV1).
	 * Requires the matching response to already exist in the indexer.
	 */
	private async buildFinalized(
		request: GetRequestWithStatus,
		hyperbridgeDelivered: RequestStatusWithMetadata,
		response: ResponseCommitmentWithValues,
	): Promise<RequestStatusWithMetadata | undefined> {
		const sourceChain = this.ctx.config.source
		const hyperbridge = this.ctx.config.hyperbridge
		const useHandlerV2 = sourceChain instanceof EvmChain && (await sourceChain.isHandlerV2())

		if (useHandlerV2) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const consensusResult = await hyperbridgeSubstrate.queryConsensusProof(
				BigInt(hyperbridgeDelivered.metadata.blockNumber),
			)
			if (!consensusResult) return undefined

			const proof = await hyperbridge.queryProof(
				{ Responses: [response.commitment as HexString] },
				request.source,
				consensusResult.provenHeight,
			)

			const calldata = sourceChain.encode({
				kind: "BatchConsensusAndGetResponse",
				consensusProof: consensusResult.proof,
				proof: {
					stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
					consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
					proof,
					height: consensusResult.provenHeight,
				},
				responses: [
					{
						get: request,
						values: request.keys.map((key, index) => ({
							key,
							value: (response.values[index] as HexString) || "0x",
						})),
					},
				],
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

		const hyperbridgeFinality = await this.queries.queryStateMachineUpdateByHeight({
			statemachineId: this.ctx.config.hyperbridge.config.stateMachineId,
			height: hyperbridgeDelivered.metadata.blockNumber,
			chain: request.source,
		})
		if (!hyperbridgeFinality) return undefined

		const proof = await hyperbridge.queryProof(
			{ Responses: [response.commitment as HexString] },
			request.source,
			BigInt(hyperbridgeFinality.height),
		)

		const calldata = sourceChain.encode({
			kind: "GetResponse",
			proof: {
				stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
				proof,
				height: BigInt(hyperbridgeFinality.height),
			},
			responses: [
				{
					get: request,
					values: request.keys.map((key, index) => ({
						key,
						value: (response.values[index] as HexString) || "0x",
					})),
				},
			],
			signer: pad("0x"),
		})

		return {
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: hyperbridgeFinality.blockHash,
				blockNumber: hyperbridgeFinality.height,
				transactionHash: hyperbridgeFinality.transactionHash,
				timestamp: hyperbridgeFinality.timestamp,
				calldata,
			},
		}
	}

	/**
	 * Streaming helper: waits (via `waitOrAbort`) for the consensus proof or
	 * state machine update, fetches the messaging proof, and returns the
	 * finalized event. Caller has already observed `HYPERBRIDGE_DELIVERED` and
	 * provides the index into `request.statuses` that carries it plus the
	 * indexed response (looked up separately because GET responses live in a
	 * different GraphQL entity from requests).
	 */
	private async streamFinalized(
		signal: AbortSignal,
		request: GetRequestWithStatus,
		hyperbridgeDeliveredIndex: number,
		response: ResponseCommitmentWithValues | undefined,
	): Promise<RequestStatusWithMetadata> {
		const sourceChain = this.ctx.config.source
		const hyperbridge = this.ctx.config.hyperbridge
		const useHandlerV2 = sourceChain instanceof EvmChain && (await sourceChain.isHandlerV2())
		const stateMachineId = this.ctx.config.hyperbridge.config.stateMachineId
		const neededHeight = BigInt(request.statuses[hyperbridgeDeliveredIndex].metadata.blockNumber)

		if (useHandlerV2) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const consensusResult = await waitOrAbort(this.ctx, {
				signal,
				promise: () => hyperbridgeSubstrate.queryConsensusProof(neededHeight),
			})

			const proof = await hyperbridge.queryProof(
				{ Responses: [response?.commitment as HexString] },
				request.source,
				consensusResult.provenHeight,
			)

			const calldata = sourceChain.encode({
				kind: "BatchConsensusAndGetResponse",
				consensusProof: consensusResult.proof,
				proof: {
					stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
					consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
					proof,
					height: consensusResult.provenHeight,
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

		const hyperbridgeFinalized = await waitOrAbort(this.ctx, {
			signal,
			promise: () =>
				this.queries.queryStateMachineUpdateByHeight({
					statemachineId: stateMachineId,
					height: Number(neededHeight),
					chain: request.source,
				}),
		})

		const proof = await hyperbridge.queryProof(
			{ Responses: [response?.commitment as HexString] },
			request.source,
			BigInt(hyperbridgeFinalized.height),
		)

		const calldata = sourceChain.encode({
			kind: "GetResponse",
			proof: {
				stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
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

		return {
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: hyperbridgeFinalized.blockHash,
				blockNumber: hyperbridgeFinalized.height,
				transactionHash: hyperbridgeFinalized.transactionHash,
				timestamp: hyperbridgeFinalized.timestamp,
				calldata,
			},
		}
	}
}
