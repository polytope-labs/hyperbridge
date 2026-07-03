import { maxBy } from "lodash-es"
import { pad } from "viem"

// @ts-ignore
import mergeRace from "@async-generator/merge-race"

import type { IGetRequestMessage, IProof, SubstrateChain } from "@/chain"
import { EvmChain } from "@/chains/evm"
import {
	type GetRequestWithStatus,
	type HexString,
	type RequestStatusKey,
	type RequestStatusWithMetadata,
	type ResponseCommitmentWithValues,
	RequestStatus,
} from "@/types"
import {
	COMBINED_STATUS_WEIGHTS,
	REQUEST_STATUS_WEIGHTS,
	getRequestCommitment,
	parseStateMachineId,
	waitForChallengePeriod,
} from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { Queries } from "./Queries"
import type { ClientContext } from "."
import { timeoutStream, waitOrAbort, withRetry } from "./utils"

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

		// Source height Hyperbridge has finalized — captured when we emit SOURCE_FINALIZED
		// and reused to build the source proof for self-delivery.
		let sourceFinalizedHeight: bigint | undefined

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

					sourceFinalizedHeight = BigInt(sourceUpdate.height)

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
					// Actively deliver the request to Hyperbridge instead of only waiting for an
					// external relayer. Best-effort: on any failure we fall back to observing, so
					// a relayer (or a retry once prerequisites are met) can still complete it.
					if (
						request.source !== this.ctx.config.hyperbridge.config.stateMachineId &&
						sourceFinalizedHeight !== undefined
					) {
						try {
							await this.deliverToHyperbridge(request, sourceFinalizedHeight)
						} catch (error) {
							this.logger.warn(
								`Self-delivery to Hyperbridge failed; waiting for a relayer instead: ${
									error instanceof Error ? error.message : error
								}`,
							)
						}
					}

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

					// The GetResponse is produced + indexed slightly after HYPERBRIDGE_DELIVERED; wait
					// for it, since streamFinalized needs its commitment to build the response proof.
					const response = await waitOrAbort(this.ctx, {
						signal,
						promise: () => this.queries.queryResponseByRequestId(hash),
					})
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
	 * Self-delivers a GET request to Hyperbridge — the request→Hyperbridge hop that would
	 * otherwise require an external relayer.
	 *
	 * Mirrors the relayer path (and {@link OrderCanceller}): prove the request commitment on
	 * the source chain at the Hyperbridge-finalized source height, prove the requested keys on
	 * the destination chain at the request's height, wait out the source challenge period, then
	 * submit an unsigned `GetRequest` message. Source and destination may each be EVM or
	 * substrate — proofs are built via the chain-agnostic {@link IChain} methods.
	 *
	 * Idempotent: returns early if Hyperbridge already holds the response receipt. The caller
	 * wraps this best-effort, so a failure leaves the stream observing as before.
	 */
	private async deliverToHyperbridge(request: GetRequestWithStatus, sourceFinalizedHeight: bigint): Promise<void> {
		const sourceChain = this.ctx.config.source
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge as SubstrateChain

		const commitment = getRequestCommitment({ ...request, keys: [...request.keys] })
		// Every network call below is retried: live RPCs (eth_getProof, WS queries, submission)
		// transiently fail (rate limits, timeouts, dropped sockets) and a single hiccup must not
		// abandon the delivery.
		const retry = { maxRetries: 5, backoffMs: 2000 }

		// Idempotency: for a GET, Hyperbridge produces the response as it handles the request,
		// so an existing response receipt (keyed by the request commitment) means it's already
		// been delivered and handled — nothing to do.
		if (await withRetry(this.ctx, () => hyperbridge.queryResponseReceipt(commitment), retry)) return

		this.logger.info(
			`Delivering GET ${commitment} to Hyperbridge (${request.source}@${sourceFinalizedHeight} → ${request.dest}@${request.height})`,
		)

		// 1. Source proof: proof of the request commitment on the source chain at the finalized
		//    height. `queryProof` builds the chain-appropriate commitment proof (EVM or substrate).
		const sourceProof: IProof = {
			height: sourceFinalizedHeight,
			stateMachine: request.source,
			consensusStateId: sourceChain.config.consensusStateId,
			proof: await withRetry(
				this.ctx,
				() =>
					sourceChain.queryProof(
						{ Requests: [commitment] },
						this.ctx.config.hyperbridge.config.stateMachineId,
						sourceFinalizedHeight,
					),
				retry,
			),
		}
		this.logger.info(`  ✓ built source proof: ${(sourceProof.proof.length - 2) / 2} bytes @ ${request.source}#${sourceFinalizedHeight}`)

		// 2. Response proof: a GET reads request.keys at exactly request.height on request.dest, and
		//    Hyperbridge enforces `proof.height == request.height`, so it must already hold a state
		//    commitment for request.dest at that exact height. Require it, then prove the keys there
		//    via the chain-appropriate state proof.
		const destCommitment = await withRetry(
			this.ctx,
			() =>
				hyperbridge.queryStateMachineCommitment({
					id: {
						stateId: parseStateMachineId(request.dest).stateId,
						consensusStateId: destChain.config.consensusStateId,
					},
					height: request.height,
				}),
			retry,
		)
		if (!destCommitment) {
			throw new Error(`Hyperbridge has no state commitment for ${request.dest} at height ${request.height}`)
		}
		const responseProof: IProof = {
			height: request.height,
			stateMachine: request.dest,
			consensusStateId: destChain.config.consensusStateId,
			proof: await withRetry(this.ctx, () => destChain.queryStateProof(request.height, [...request.keys]), retry),
		}
		this.logger.info(
			`  ✓ built response proof: ${(responseProof.proof.length - 2) / 2} bytes (${request.keys.length} key(s) @ ${request.dest}#${request.height})`,
		)

		// 3. Wait out the source challenge period on Hyperbridge.
		await withRetry(
			this.ctx,
			() =>
				waitForChallengePeriod(hyperbridge, {
					height: sourceFinalizedHeight,
					id: {
						stateId: parseStateMachineId(request.source).stateId,
						consensusStateId: sourceChain.config.consensusStateId,
					},
				}),
			retry,
		)

		// 4. Submit the unsigned GetRequest message.
		const message: IGetRequestMessage = {
			kind: "GetRequest",
			requests: [
				{
					source: request.source,
					dest: request.dest,
					nonce: request.nonce,
					from: request.from,
					timeoutTimestamp: request.timeoutTimestamp,
					keys: [...request.keys],
					height: request.height,
					context: request.context,
				},
			],
			source: sourceProof,
			response: responseProof,
			signer: pad("0x"),
		}

		this.logger.info("  → submitting GetRequest message (source + response proofs) to Hyperbridge…")
		// Idempotent via the response-receipt check above; retry so a dropped socket / timeout on
		// submission doesn't strand the request waiting for a relayer that never comes.
		const result = await withRetry(this.ctx, () => hyperbridge.submitUnsigned(message), retry)
		this.logger.info(
			`  ✓ delivered GET ${commitment} in Hyperbridge block #${result.blockNumber} (tx ${result.transactionHash})`,
		)
	}

	/**
	 * Snapshot helper: returns the `HYPERBRIDGE_FINALIZED` event with source-chain
	 * calldata if prerequisites are met, or `undefined` if we're still waiting
	 * for a consensus proof.
	 * Requires the matching response to already exist in the indexer.
	 */
	private async buildFinalized(
		request: GetRequestWithStatus,
		hyperbridgeDelivered: RequestStatusWithMetadata,
		response: ResponseCommitmentWithValues,
	): Promise<RequestStatusWithMetadata | undefined> {
		const sourceChain = this.ctx.config.source
		const hyperbridge = this.ctx.config.hyperbridge
		const { config } = hyperbridge

		// Has the response's destination chain (a GET returns to its `source`) already finalized
		// Hyperbridge at >= the delivered height? That's what `handleGetResponses` verifies against, so if
		// so we can deliver a plain GetResponse proof — no consensus proof needed.
		const finality = await this.queries.queryStateMachineUpdateByHeight({
			statemachineId: config.stateMachineId,
			height: hyperbridgeDelivered.metadata.blockNumber,
			chain: request.source,
		})
		if (finality) {
			const proof = await hyperbridge.queryProof(
				{ Responses: [response.commitment as HexString] },
				request.source,
				BigInt(finality.height),
			)
			const calldata = sourceChain.encode({
				kind: "GetResponse",
				proof: {
					stateMachine: config.stateMachineId,
					consensusStateId: config.consensusStateId,
					proof,
					height: BigInt(finality.height),
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
					blockHash: finality.blockHash,
					blockNumber: finality.height,
					transactionHash: finality.transactionHash,
					timestamp: finality.timestamp,
					calldata,
				},
			}
		}

		// No existing finality. For an EVM source, advance its Hyperbridge light client and deliver in
		// one batch (consensus proof + message proof).
		if (sourceChain instanceof EvmChain) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const currentEpoch = await sourceChain.currentEpoch()
			const consensusResult = await hyperbridgeSubstrate.queryConsensusProofs(
				BigInt(hyperbridgeDelivered.metadata.blockNumber),
				currentEpoch,
			)
			if (!consensusResult) return undefined

			const proof = await hyperbridge.queryProof(
				{ Responses: [response.commitment as HexString] },
				request.source,
				consensusResult.provenHeight,
			)

			const calldata = sourceChain.encode({
				kind: "BatchConsensusAndGetResponse",
				consensusProofs: consensusResult.proofs,
				proof: {
					stateMachine: config.stateMachineId,
					consensusStateId: config.consensusStateId,
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

			// HYPERBRIDGE_FINALIZED anchors to Hyperbridge's own state-machine update (its self-finality of
			// the delivered height), not the delivery event. Snapshot: bail if it isn't indexed yet.
			const hyperbridgeFinality = await this.queries.queryStateMachineUpdateByHeight({
				statemachineId: config.stateMachineId,
				height: hyperbridgeDelivered.metadata.blockNumber,
				chain: config.stateMachineId,
			})
			if (!hyperbridgeFinality) return undefined
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

		return undefined
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
		const stateMachineId = this.ctx.config.hyperbridge.config.stateMachineId
		const neededHeight = BigInt(request.statuses[hyperbridgeDeliveredIndex].metadata.blockNumber)

		const consensusStateId = this.ctx.config.hyperbridge.config.consensusStateId
		const encodeGetResponse = (height: bigint, proof: HexString): HexString =>
			sourceChain.encode({
				kind: "GetResponse",
				proof: { stateMachine: stateMachineId, consensusStateId, proof, height },
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

		// Has the response's destination chain (a GET returns to its `source`) already finalized
		// Hyperbridge at a height >= neededHeight? That's exactly what `handleGetResponses` verifies
		// against (host.stateMachineCommitment(proof.height)), and Hyperbridge's overlay MMR is
		// append-only, so any such update proves the response. Read it from the indexer (no per-call
		// RPC): if present, deliver a plain GetResponse — the destination host already holds the
		// commitment. Otherwise (EVM source) advance the host's Hyperbridge client with a consensus proof.
		let finality = await this.queries.queryStateMachineUpdateByHeight({
			statemachineId: stateMachineId,
			height: Number(neededHeight),
			chain: request.source,
		})
		if (finality) {
			const proof = await hyperbridge.queryProof(
				{ Responses: [response?.commitment as HexString] },
				request.source,
				BigInt(finality.height),
			)
			return {
				status: RequestStatus.HYPERBRIDGE_FINALIZED,
				metadata: {
					blockHash: finality.blockHash,
					blockNumber: finality.height,
					transactionHash: finality.transactionHash,
					timestamp: finality.timestamp,
					calldata: encodeGetResponse(BigInt(finality.height), proof),
				},
			}
		}

		// Not yet finalized on the destination and the source is EVM: advance its Hyperbridge light
		// client and deliver in one batch (consensus proof + response proof).
		if (sourceChain instanceof EvmChain) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const currentEpoch = await sourceChain.currentEpoch()
			const consensusResult = await waitOrAbort(this.ctx, {
				signal,
				promise: () => hyperbridgeSubstrate.queryConsensusProofs(neededHeight, currentEpoch),
			})

			const proof = await hyperbridge.queryProof(
				{ Responses: [response?.commitment as HexString] },
				request.source,
				consensusResult.provenHeight,
			)

			const calldata = sourceChain.encode({
				kind: "BatchConsensusAndGetResponse",
				consensusProofs: consensusResult.proofs,
				proof: {
					stateMachine: stateMachineId,
					consensusStateId,
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

			// HYPERBRIDGE_FINALIZED anchors to Hyperbridge's own state-machine update (its self-finality of
			// neededHeight) — the batch carries that finality to the destination, so the delivery event is
			// the wrong anchor. Wait for the self-update to be indexed.
			const hyperbridgeFinality = await waitOrAbort(this.ctx, {
				signal,
				promise: () =>
					this.queries.queryStateMachineUpdateByHeight({
						statemachineId: stateMachineId,
						height: Number(neededHeight),
						chain: stateMachineId,
					}),
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

		// Non-EVM source: wait for the destination chain to finalize Hyperbridge, then a plain proof.
		finality = await waitOrAbort(this.ctx, {
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
			BigInt(finality.height),
		)

		return {
			status: RequestStatus.HYPERBRIDGE_FINALIZED,
			metadata: {
				blockHash: finality.blockHash,
				blockNumber: finality.height,
				transactionHash: finality.transactionHash,
				timestamp: finality.timestamp,
				calldata: encodeGetResponse(BigInt(finality.height), proof),
			},
		}
	}
}
