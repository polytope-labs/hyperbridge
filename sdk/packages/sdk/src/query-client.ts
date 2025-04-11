import { GraphQLClient } from "graphql-request"
import { DEFAULT_LOGGER, REQUEST_STATUS_WEIGHTS, retryPromise } from "./utils"
import type {
	GetRequestResponse,
	GetRequestWithStatus,
	IndexerQueryClient,
	RequestResponse,
	RequestStatusKey,
	PostRequestWithStatus,
} from "./types"
import type { ConsolaInstance } from "consola"
import { GET_REQUEST_STATUS, POST_REQUEST_STATUS } from "./queries"

export function createQueryClient(config: { url: string }) {
	return new GraphQLClient(config.url)
}

/**
 * Queries a request by CommitmentHash
 *
 * @example
 * import { createQueryClient, queryRequest } from "hyperbridge-sdk"
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
 * import { createQueryClient, queryRequest } from "hyperbridge-sdk"
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
		statuses: sorted,
	}
}
