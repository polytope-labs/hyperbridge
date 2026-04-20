import type {
	GetRequestWithStatus,
	GetResponseByRequestIdResponse,
	HexString,
	PostRequestWithStatus,
	ResponseCommitmentWithValues,
	StateMachineResponse,
	StateMachineUpdate,
} from "@/types"
import {
	_queryGetRequestInternal,
	_queryRequestInternal,
} from "@/query-client"
import {
	GET_RESPONSE_BY_REQUEST_ID,
	LATEST_STATE_MACHINE_UPDATE,
	STATE_MACHINE_UPDATES_BY_HEIGHT,
	STATE_MACHINE_UPDATES_BY_HEIGHT_DESC,
	STATE_MACHINE_UPDATES_BY_TIMESTAMP,
} from "@/queries"
import { dateStringtoTimestamp } from "@/utils"

import type { ClientContext } from "./types"
import { withRetry } from "./utils"

/**
 * Read-only indexer queries — state machine updates, post/get requests, and
 * response lookups. Groups the thin GraphQL wrappers that other client
 * sub-modules consume when building finality events and status streams.
 */
export class StateMachineQueries {
	private readonly logger

	constructor(private readonly ctx: ClientContext) {
		this.logger = ctx.logger.withTag("[StateMachineQueries]")
	}

	/**
	 * Query for a single state machine update event greater than or equal to the given height.
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

		// Query both ASC (for earliest timestamp) and DESC (for latest state machine height)
		const [ascResponse, descResponse] = await Promise.all([
			withRetry(
				this.ctx,
				() =>
					this.ctx.graphql.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_HEIGHT, {
						statemachineId,
						height,
						chain,
					}),
			),
			withRetry(
				this.ctx,
				() =>
					this.ctx.graphql.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_HEIGHT_DESC, {
						statemachineId,
						height,
						chain,
					}),
			),
		])

		const ascNode = ascResponse?.stateMachineUpdateEvents?.nodes[0]
		const descNode = descResponse?.stateMachineUpdateEvents?.nodes[0]

		if (!ascNode) return undefined

		const timestamp = Math.floor(dateStringtoTimestamp(ascNode.createdAt) / 1000)
		const stateMachineHeight = descNode?.height ?? ascNode.height

		const combined: StateMachineUpdate = {
			height: stateMachineHeight,
			chain: ascNode.chain,
			blockHash: ascNode.blockHash,
			blockNumber: ascNode.blockNumber,
			transactionHash: ascNode.transactionHash,
			transactionIndex: ascNode.transactionIndex,
			stateMachineId: ascNode.stateMachineId,
			timestamp,
		}

		logger.trace(`${message} -> response`, combined)
		return combined
	}

	/**
	 * Query for a single state machine update event greater than or equal to the given timestamp.
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

		const response = await withRetry(this.ctx, () =>
			this.ctx.graphql.request<StateMachineResponse>(STATE_MACHINE_UPDATES_BY_TIMESTAMP, {
				statemachineId,
				commitmentTimestamp: commitmentTimestamp.toString(),
				chain,
			}),
		)

		const first_node = response?.stateMachineUpdateEvents?.nodes[0]
		if (first_node?.createdAt) {
			// @ts-ignore
			first_node.timestamp = Math.floor(dateStringtoTimestamp(first_node.createdAt) / 1000)
		}
		logger.trace("Response >", first_node)

		// @ts-ignore
		return first_node
	}

	/**
	 * Query for the latest state machine update height.
	 */
	async queryLatestStateMachineHeight({
		statemachineId,
		chain,
	}: {
		statemachineId: string
		chain: string
	}): Promise<bigint | undefined> {
		const logger = this.logger.withTag("[queryLatestStateMachineHeight]()")

		const response = await withRetry(this.ctx, () =>
			this.ctx.graphql.request<StateMachineResponse>(LATEST_STATE_MACHINE_UPDATE, {
				statemachineId,
				chain,
			}),
		)

		const first_node = response?.stateMachineUpdateEvents?.nodes[0]
		if (!first_node) return undefined

		logger.trace("Latest height >", first_node.height)
		return BigInt(first_node.height)
	}

	/**
	 * Queries a POST request by commitment hash.
	 */
	async queryPostRequest(commitmentHash: HexString): Promise<PostRequestWithStatus | undefined> {
		return _queryRequestInternal({
			commitmentHash,
			queryClient: this.ctx.graphql,
			logger: this.ctx.logger,
		})
	}

	/**
	 * Queries a GET request by any of its associated hashes.
	 */
	async queryGetRequest(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		return _queryGetRequestInternal({
			commitmentHash: hash,
			queryClient: this.ctx.graphql,
			logger: this.ctx.logger,
		})
	}

	/**
	 * Queries the response associated with a specific request ID and returns its commitment.
	 */
	async queryResponseByRequestId(requestId: string): Promise<ResponseCommitmentWithValues | undefined> {
		const response = await withRetry(this.ctx, () =>
			this.ctx.graphql.request<GetResponseByRequestIdResponse>(GET_RESPONSE_BY_REQUEST_ID, { requestId }),
		)

		if (!response.getResponses.nodes.length) return undefined

		const firstResponse = response.getResponses.nodes[0]
		return {
			commitment: firstResponse.commitment,
			values: firstResponse.responseMessage,
		}
	}
}
