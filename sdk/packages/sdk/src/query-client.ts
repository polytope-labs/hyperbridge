import { GraphQLClient } from "graphql-request"
import { DEFAULT_LOGGER, REQUEST_STATUS_WEIGHTS, retryPromise } from "./utils"
import type {
	GetRequestResponse,
	GetRequestWithStatus,
	IndexerQueryClient,
	RequestResponse,
	RequestStatusKey,
	PostRequestWithStatus,
	OrderResponse,
	OrderWithStatus,
	TokenGatewayAssetTeleportedResponse,
	TokenGatewayAssetTeleportedWithStatus,
} from "./types"
import type { ConsolaInstance } from "consola"
import { GET_REQUEST_STATUS, POST_REQUEST_STATUS, ORDER_STATUS, TOKEN_GATEWAY_ASSET_TELEPORTED_STATUS } from "./queries"

export function createQueryClient(config: { url: string }) {
	return new GraphQLClient(config.url)
}

/**
 * Queries a request by CommitmentHash
 *
 * @example
 * import { createQueryClient, queryRequest } from "@hyperbridge/sdk"
 *
 * const queryClient = createQueryClient({
 *   url: "http://localhost:3000", // URL of the Hyperbridge indexer API
 * })
 * const commitmentHash = "0x...."
 * const request = await queryPostRequest({ commitmentHash, queryClient })
 */
export function queryPostRequest(params: { commitmentHash: string; queryClient: IndexerQueryClient }) {
	return _queryRequestInternal(params)
}

/**
 * Queries a GET Request by CommitmentHash
 *
 * @example
 * import { createQueryClient, queryRequest } from "@hyperbridge/sdk"
 *
 * const queryClient = createQueryClient({
 *   url: "http://localhost:3000", // URL of the Hyperbridge indexer API
 * })
 * const commitmentHash = "0x...."
 * const request = await queryGetRequest({ commitmentHash, queryClient })
 */
export function queryGetRequest(params: { commitmentHash: string; queryClient: IndexerQueryClient }) {
	return _queryGetRequestInternal(params)
}

/**
 * Queries an order by CommitmentHash
 *
 * @example
 * import { createQueryClient, queryOrder } from "@hyperbridge/sdk"
 *
 * const queryClient = createQueryClient({
 *   url: "http://localhost:3000", // URL of the Hyperbridge indexer API
 * })
 * const commitmentHash = "0x...."
 * const order = await queryOrder({ commitmentHash, queryClient })
 */
export function queryOrder(params: { commitmentHash: string; queryClient: IndexerQueryClient }) {
	return _queryOrderInternal(params)
}

/**
 * Internal function to query a token gateway asset teleported by CommitmentHash
 *
 * @param params - Parameters for querying the token gateway asset teleported
 * @returns Latest status and block metadata of the token gateway asset teleported
 */
export async function _queryTokenGatewayAssetTeleportedInternal(
	params: InternalQueryParams,
): Promise<TokenGatewayAssetTeleportedWithStatus | undefined> {
	const { commitmentHash, queryClient: client, logger = DEFAULT_LOGGER } = params

	const response = await retryPromise(
		() => {
			return client.request<TokenGatewayAssetTeleportedResponse>(TOKEN_GATEWAY_ASSET_TELEPORTED_STATUS, {
				commitment: commitmentHash,
			})
		},
		{
			maxRetries: 3,
			backoffMs: 1000,
			logger,
			logMessage: `querying 'TokenGatewayAssetTeleported' with Statuses by CommitmentHash(${commitmentHash})`,
		},
	)

	const first_record = response.tokenGatewayAssetTeleporteds.nodes[0]
	if (!first_record) return

	logger.trace("`TokenGatewayAssetTeleported` found")
	const { statusMetadata, ...first_node } = first_record

	const statuses = structuredClone(statusMetadata.nodes).map((item) => ({
		status: item.status,
		metadata: {
			blockHash: item.blockHash,
			blockNumber: Number.parseInt(item.blockNumber),
			transactionHash: item.transactionHash,
			timestamp: BigInt(item.timestamp),
		},
	}))

	// sort by ascending order
	const sorted = statuses.sort((a, b) => {
		return Number(a.metadata.timestamp) - Number(b.metadata.timestamp)
	})

	return {
		...first_node,
		amount: BigInt(first_node.amount),
		blockNumber: BigInt(first_node.blockNumber),
		blockTimestamp: BigInt(first_node.blockTimestamp),
		createdAt: new Date(first_node.createdAt),
		statuses: sorted,
	}
}

type InternalQueryParams = {
	commitmentHash: string
	queryClient: IndexerQueryClient
	logger?: ConsolaInstance
}

/**
  * Queries a request by CommitmentHash

  * @param hash - Can be commitment
  * @returns Latest status and block metadata of the request
  */
export async function _queryRequestInternal(params: InternalQueryParams): Promise<PostRequestWithStatus | undefined> {
	const { commitmentHash: hash, queryClient: client, logger: logger_ = DEFAULT_LOGGER } = params

	const logger = logger_.withTag("[queryRequest]")

	const response = await retryPromise(
		() => {
			return client.request<RequestResponse>(POST_REQUEST_STATUS, {
				hash,
			})
		},
		{
			maxRetries: 3,
			backoffMs: 1000,
			logger,
			logMessage: `querying 'Request' with Statuses by CommitmentHash(${hash})`,
		},
	)

	const first_record = response.requests.nodes[0]
	if (!first_record) return

	logger.trace("`Request` found")
	const { statusMetadata, ...first_node } = first_record

	const statuses = structuredClone(statusMetadata.nodes).map((item) => ({
		status: item.status as any,
		metadata: {
			blockHash: item.blockHash,
			blockNumber: Number.parseInt(item.blockNumber),
			transactionHash: item.transactionHash,
			timestamp: item?.timestamp,
		},
	}))

	// sort by ascending order
	const sorted = statuses.sort(
		(a, b) =>
			REQUEST_STATUS_WEIGHTS[a.status as RequestStatusKey] - REQUEST_STATUS_WEIGHTS[b.status as RequestStatusKey],
	)
	logger.trace("Statuses found", statuses)

	const request: PostRequestWithStatus = {
		...first_node,
		timeoutTimestamp: BigInt(first_node.timeoutTimestamp),
		statuses: sorted,
	}

	return request
}

/**
 * Queries a request by any of its associated hashes and returns it alongside its statuses
 * Statuses will be one of SOURCE, HYPERBRIDGE_DELIVERED and DESTINATION
 *
 * @param hash - Can be commitment, hyperbridge tx hash, source tx hash, destination tx hash, or timeout tx hash
 * @returns Latest status and block metadata of the request
 */
export async function _queryGetRequestInternal(params: InternalQueryParams): Promise<GetRequestWithStatus | undefined> {
	const { commitmentHash, queryClient: client, logger = DEFAULT_LOGGER } = params

	const response = await retryPromise(
		() => {
			return client.request<GetRequestResponse>(GET_REQUEST_STATUS, {
				commitment: commitmentHash,
			})
		},
		{
			maxRetries: 3,
			backoffMs: 1000,
			logger,
			logMessage: `query \`IGetRequest\` with commitment hash ${commitmentHash}`,
		},
	)

	if (!response.getRequests.nodes[0]) return

	logger.trace("`Request` found")

	const statuses = response.getRequests.nodes[0].statusMetadata.nodes.map((item) => ({
		status: item.status as any,
		metadata: {
			blockHash: item.blockHash,
			blockNumber: Number.parseInt(item.blockNumber),
			transactionHash: item.transactionHash,
			timestamp: item?.timestamp,
		},
	}))

	// sort by ascending order
	const sorted = statuses.sort((a, b) => {
		return (
			REQUEST_STATUS_WEIGHTS[a.status as RequestStatusKey] - REQUEST_STATUS_WEIGHTS[b.status as RequestStatusKey]
		)
	})

	const { statusMetadata, ...rest } = response.getRequests.nodes[0]

	return {
		...rest,
		timeoutTimestamp: BigInt(rest.timeoutTimestamp),
		nonce: BigInt(rest.nonce),
		height: BigInt(rest.height),
		statuses: sorted,
	}
}

/**
 * Internal function to query an order by CommitmentHash
 *
 * @param params - Parameters for querying the order
 * @returns Latest status and block metadata of the order
 */
export async function _queryOrderInternal(params: InternalQueryParams): Promise<OrderWithStatus | undefined> {
	const { commitmentHash, queryClient: client, logger = DEFAULT_LOGGER } = params

	const response = await retryPromise(
		() => {
			return client.request<OrderResponse>(ORDER_STATUS, {
				commitment: commitmentHash,
			})
		},
		{
			maxRetries: 3,
			backoffMs: 1000,
			logger,
			logMessage: `querying 'Order' with Statuses by CommitmentHash(${commitmentHash})`,
		},
	)

	const first_record = response.orderPlaceds.nodes[0]
	if (!first_record) return

	logger.trace("`Order` found")
	const { statusMetadata, ...first_node } = first_record

	const statuses = structuredClone(statusMetadata.nodes).map((item) => ({
		status: item.status,
		metadata: {
			blockHash: item.blockHash,
			blockNumber: Number.parseInt(item.blockNumber),
			transactionHash: item.transactionHash,
			timestamp: BigInt(item.timestamp),
			filler: item.filler,
		},
	}))

	// sort by ascending order
	const sorted = statuses.sort((a, b) => {
		// Since OrderStatus and RequestStatus are different enums, we'll just sort by timestamp
		return Number(a.metadata.timestamp) - Number(b.metadata.timestamp)
	})

	const order: OrderWithStatus = {
		...first_node,
		deadline: BigInt(first_node.deadline),
		nonce: BigInt(first_node.nonce),
		fees: BigInt(first_node.fees),
		inputAmounts: first_node.inputAmounts.map(BigInt),
		outputAmounts: first_node.outputAmounts.map(BigInt),
		blockNumber: BigInt(first_node.blockNumber),
		blockTimestamp: BigInt(first_node.blockTimestamp),
		createdAt: new Date(first_node.createdAt),
		statuses: sorted,
	}

	return order
}
