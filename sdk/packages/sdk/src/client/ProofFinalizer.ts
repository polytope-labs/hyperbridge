import { pad } from "viem"

import type { SubstrateChain } from "@/chain"
import { EvmChain } from "@/chains/evm"
import {
	type GetRequestWithStatus,
	type HexString,
	type PostRequestWithStatus,
	type RequestStatusWithMetadata,
	type ResponseCommitmentWithValues,
	RequestStatus,
} from "@/types"
import { parseStateMachineId, postRequestCommitment, waitForChallengePeriod } from "@/utils"

import type { StateMachineQueries } from "./StateMachineQueries"
import type { ClientContext } from "./types"
import { waitOrAbort, withRetry } from "./utils"

/**
 * Builds the `HYPERBRIDGE_FINALIZED` status event for a request, selecting between
 * the HandlerV1 path (waits for an explicit consensus message to land on the
 * counterparty chain) and the HandlerV2 path (pulls the consensus proof from
 * Hyperbridge's offchain storage and encodes a single `batchCall` with the
 * messaging proof).
 *
 * Collapses four near-identical code paths that used to live inline across
 * the POST / GET snapshot and streaming flows.
 */
export class ProofFinalizer {
	private readonly logger

	constructor(
		private readonly ctx: ClientContext,
		private readonly queries: StateMachineQueries,
	) {
		this.logger = ctx.logger.withTag("[ProofFinalizer]")
	}

	/**
	 * Snapshot builder for POST requests. Returns the `HYPERBRIDGE_FINALIZED` event
	 * if all prerequisites are met, or `undefined` if we're still waiting for a
	 * consensus proof (HandlerV2) or state machine update (HandlerV1).
	 */
	async buildPostRequestFinalized(
		request: PostRequestWithStatus,
		hyperbridgeDelivered: RequestStatusWithMetadata,
	): Promise<RequestStatusWithMetadata | undefined> {
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const useHandlerV2 = destChain instanceof EvmChain && (await destChain.isHandlerV2())

		if (useHandlerV2) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const consensusResult = await hyperbridgeSubstrate.queryConsensusProof(
				BigInt(hyperbridgeDelivered.metadata.blockNumber),
			)
			if (!consensusResult) return undefined

			const proof = await hyperbridge.queryProof(
				{ Requests: [postRequestCommitment(request).commitment] },
				request.dest,
				consensusResult.provenHeight,
			)

			const calldata = destChain.encode({
				kind: "BatchConsensusAndPostRequest",
				consensusProof: consensusResult.proof,
				proof: {
					stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
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
			chain: request.dest,
		})
		if (!hyperbridgeFinality) return undefined

		const proof = await hyperbridge.queryProof(
			{ Requests: [postRequestCommitment(request).commitment] },
			request.dest,
			BigInt(hyperbridgeFinality.height),
		)

		const calldata = destChain.encode({
			kind: "PostRequest",
			proof: {
				stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
				proof,
				height: BigInt(hyperbridgeFinality.height),
			},
			requests: [request],
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
	 * Snapshot builder for GET responses. Requires the matching response to already
	 * exist in the indexer.
	 */
	async buildGetResponseFinalized(
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
	 * Streaming builder for POST requests — waits (via `waitOrAbort`) for the
	 * consensus proof or state machine update, fetches the messaging proof, and
	 * returns the finalized event. Caller has already observed `HYPERBRIDGE_DELIVERED`
	 * and provides the index into `request.statuses` that carries it.
	 */
	async streamPostRequestFinalized(
		signal: AbortSignal,
		request: PostRequestWithStatus,
		hyperbridgeDeliveredIndex: number,
	): Promise<RequestStatusWithMetadata> {
		const destChain = this.ctx.config.dest
		const hyperbridge = this.ctx.config.hyperbridge
		const useHandlerV2 = destChain instanceof EvmChain && (await destChain.isHandlerV2())
		const stateMachineId = this.ctx.config.hyperbridge.config.stateMachineId
		const neededHeight = BigInt(request.statuses[hyperbridgeDeliveredIndex].metadata.blockNumber)

		if (useHandlerV2) {
			const hyperbridgeSubstrate = hyperbridge as SubstrateChain
			const consensusResult = await waitOrAbort(this.ctx, {
				signal,
				promise: () => hyperbridgeSubstrate.queryConsensusProof(neededHeight),
			})

			const proof = await this.fetchProofWithRetry(signal, () =>
				hyperbridge.queryProof(
					{ Requests: [postRequestCommitment(request).commitment] },
					request.dest,
					consensusResult.provenHeight,
				),
			)

			const calldata = destChain.encode({
				kind: "BatchConsensusAndPostRequest",
				consensusProof: consensusResult.proof,
				proof: {
					stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
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

		const hyperbridgeFinalized = await waitOrAbort(this.ctx, {
			signal,
			promise: () =>
				this.queries.queryStateMachineUpdateByHeight({
					statemachineId: stateMachineId,
					height: Number(neededHeight),
					chain: request.dest,
				}),
		})

		const proof = await this.fetchProofWithRetry(signal, () =>
			hyperbridge.queryProof(
				{ Requests: [postRequestCommitment(request).commitment] },
				request.dest,
				BigInt(hyperbridgeFinalized.height),
			),
		)

		const calldata = destChain.encode({
			kind: "PostRequest",
			proof: {
				stateMachine: this.ctx.config.hyperbridge.config.stateMachineId,
				consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId,
				proof,
				height: BigInt(hyperbridgeFinalized.height),
			},
			requests: [request],
			signer: pad("0x"),
		})

		const { stateId } = parseStateMachineId(stateMachineId)
		await waitForChallengePeriod(destChain, {
			height: BigInt(hyperbridgeFinalized.height),
			id: { stateId, consensusStateId: this.ctx.config.hyperbridge.config.consensusStateId },
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

	/**
	 * Streaming builder for GET responses. Mirrors {@link streamPostRequestFinalized}
	 * but against the source chain (responses travel back to the origin) and reads
	 * the response out of the indexer first.
	 */
	async streamGetResponseFinalized(
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

	/**
	 * Retry-wrapped `hyperbridge.queryProof` for streaming flows. Fails after 6
	 * retries (~2 minutes at 2s backoff) so a hung Hyperbridge node doesn't stall
	 * the stream forever.
	 */
	private async fetchProofWithRetry(signal: AbortSignal, fetch: () => Promise<HexString>): Promise<HexString> {
		const safe = async () => {
			try {
				return { data: await fetch(), error: null as unknown }
			} catch (err) {
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
}
