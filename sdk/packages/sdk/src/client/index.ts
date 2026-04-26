import { type ConsolaInstance, LogLevels, createConsola } from "consola"

import type { IChain, SubstrateChain } from "@/chain"
import type {
	ClientConfig,
	GetRequestWithStatus,
	HexString,
	IndexerQueryClient,
	PostRequestTimeoutStatus,
	PostRequestWithStatus,
	RequestStatusWithMetadata,
	RetryConfig,
	StateMachineUpdate,
} from "@/types"
import { DEFAULT_POLL_INTERVAL } from "@/utils"

import { GetRequestClient } from "./GetRequestClient"
import { PostRequestClient } from "./PostRequestClient"
import { Queries } from "./Queries"
import { timeoutStream } from "./utils"

/**
 * Shared dependencies passed to every sub-module of {@link IsmpClient}.
 *
 * Sub-modules read their chain configuration, GraphQL client, logger, and
 * retry helpers off of this object rather than reaching back into the facade.
 *
 * Internal — not part of the public SDK surface.
 */
export interface ClientContext {
	/** Chain configuration + indexer poll interval. `pollInterval` is guaranteed set. */
	config: ClientConfig & { pollInterval: number }
	/** GraphQL client used to query the Hyperbridge indexer. */
	graphql: IndexerQueryClient
	/** Structured logger (tag-scoped per sub-module). */
	logger: ConsolaInstance
	/** Default retry config — sub-modules may override per-call. */
	defaultRetryConfig: RetryConfig
}

/**
 * `IsmpClient` tracks ISMP POST/GET requests and their timeout flows across
 * source, Hyperbridge, and destination chains.
 *
 * The client:
 * - queries the Hyperbridge indexer for request status,
 * - augments status trails with derived finality events (`SOURCE_FINALIZED`,
 *   `HYPERBRIDGE_FINALIZED`, timeout events),
 * - streams live updates while a request is in flight, and
 * - generates the calldata a relayer needs to finalize a request on the
 *   counterparty chain (including the HandlerV2 batch path).
 *
 * The RPC URLs in {@link ClientConfig} must point to archive nodes — timeout
 * and challenge-period flows reach back into older blocks for storage proofs.
 *
 * @example
 * ```typescript
 * const client = new IsmpClient({
 *   queryClient,
 *   source: sourceChain,
 *   dest: destChain,
 *   hyperbridge: hyperbridgeChain,
 *   pollInterval: 2000,
 * })
 *
 * const status = await client.queryRequestWithStatus("0x1234...")
 *
 * for await (const update of client.postRequestStatusStream("0x1234...")) {
 *   console.log(`Request status: ${update.status}`)
 * }
 * ```
 */
export class IsmpClient {
	/** Shared context threaded through every sub-module. */
	protected readonly ctx: ClientContext
	/** Indexer read-only queries (state machine updates, post/get requests). */
	protected readonly queries: Queries
	/** POST request lifecycle: snapshot + streaming status, timeout flow, aggregation. */
	protected readonly postRequest: PostRequestClient
	/** GET request snapshot + streaming status. */
	protected readonly getRequest: GetRequestClient

	constructor(config: PartialClientConfig) {
		const logger = createConsola({
			level: LogLevels[config.tracing ? "trace" : "info"],
			formatOptions: { columns: 80, colors: true, compact: true, date: false },
		})

		const defaultRetryConfig: RetryConfig = { maxRetries: 3, backoffMs: 1000 }

		this.ctx = {
			config: { pollInterval: DEFAULT_POLL_INTERVAL, ...config },
			graphql: config.queryClient,
			logger,
			defaultRetryConfig,
		}

		this.queries = new Queries(this.ctx)
		this.postRequest = new PostRequestClient(this.ctx, this.queries)
		this.getRequest = new GetRequestClient(this.ctx, this.queries)
	}

	/** Source chain instance. */
	get source(): IChain {
		return this.ctx.config.source
	}
	/** Destination chain instance. */
	get dest(): IChain {
		return this.ctx.config.dest
	}
	/** Hyperbridge chain instance. */
	get hyperbridge(): IChain {
		return this.ctx.config.hyperbridge
	}

	/**
	 * Query for the first state machine update at or after `height` for the given
	 * state machine on the indicated counterparty chain.
	 *
	 * @returns The matching update, or `undefined` if none has been indexed yet.
	 */
	queryStateMachineUpdateByHeight(args: {
		/** State machine id whose commitment is being looked up (e.g. `EVM-1`). */
		statemachineId: string
		/** Chain id where the update was applied (e.g. `POLKADOT-3367`). */
		chain: string
		/** Lower bound on the update height. */
		height: number
	}): Promise<StateMachineUpdate | undefined> {
		return this.queries.queryStateMachineUpdateByHeight(args)
	}

	/**
	 * Query for the first state machine update whose commitment timestamp is at
	 * or after `commitmentTimestamp`. Used by timeout flows to find when a
	 * counterparty chain finalized a state past the request's deadline.
	 */
	queryStateMachineUpdateByTimestamp(args: {
		statemachineId: string
		commitmentTimestamp: bigint
		chain: string
	}): Promise<StateMachineUpdate | undefined> {
		return this.queries.queryStateMachineUpdateByTimestamp(args)
	}

	/**
	 * Returns the latest known state machine height for a given state machine,
	 * or `undefined` if the indexer has no record yet.
	 */
	queryLatestStateMachineHeight(args: { statemachineId: string; chain: string }): Promise<bigint | undefined> {
		return this.queries.queryLatestStateMachineHeight(args)
	}

	/**
	 * Queries a POST request by any of its associated hashes (commitment,
	 * source/destination/hyperbridge tx hash, or timeout tx hash). Returns the
	 * raw indexed record without derived finality events.
	 */
	queryPostRequest(commitmentHash: HexString): Promise<PostRequestWithStatus | undefined> {
		return this.queries.queryPostRequest(commitmentHash)
	}

	/**
	 * Queries a GET request by any of its associated hashes. Returns the raw
	 * indexed record without derived finality events.
	 */
	queryGetRequest(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		return this.queries.queryGetRequest(hash)
	}

	/**
	 * Queries the response a GET request's values come from, by the request's id.
	 * Returns the response commitment plus the indexed storage values.
	 */
	queryResponseByRequestId(requestId: string) {
		return this.queries.queryResponseByRequestId(requestId)
	}

	/**
	 * Snapshot query for a POST request: returns the indexed record with all
	 * inferred finality and timeout events (`SOURCE_FINALIZED`,
	 * `HYPERBRIDGE_FINALIZED`, `PENDING_TIMEOUT`, …) sorted by progress weight.
	 *
	 * Unlike {@link postRequestStatusStream} this does not poll; each call
	 * returns a fresh snapshot based on the indexer's current state.
	 */
	queryRequestWithStatus(hash: HexString): Promise<PostRequestWithStatus | undefined> {
		return this.postRequest.queryRequestWithStatus(hash)
	}

	/**
	 * Streams status updates for a POST request as it progresses through the
	 * source → Hyperbridge → destination lifecycle. Ends when the request
	 * reaches its destination or its timeout becomes pending.
	 *
	 * Yields the `HYPERBRIDGE_FINALIZED` event with relayer calldata attached —
	 * this is the calldata a caller submits to the destination handler to
	 * deliver the request (bundled `handleConsensus` + `handlePostRequests` via
	 * `batchCall`).
	 */
	postRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		return this.postRequest.postRequestStatusStream(hash)
	}

	/**
	 * Streams timeout status updates for a POST request past its
	 * `timeoutTimestamp`. Drives the `PENDING_TIMEOUT →
	 * DESTINATION_FINALIZED_TIMEOUT → HYPERBRIDGE_FINALIZED_TIMEOUT → TIMED_OUT`
	 * progression, submitting unsigned extrinsics to Hyperbridge where required
	 * and yielding source-chain timeout calldata at the final step.
	 */
	postRequestTimeoutStream(hash: HexString): AsyncGenerator<PostRequestTimeoutStatus, void> {
		return this.postRequest.postRequestTimeoutStream(hash)
	}

	/**
	 * Relays a POST request to Hyperbridge from source-chain state (when the
	 * request wasn't aggregated there yet) and returns the finalized Hyperbridge
	 * extrinsic receipt. Used to bootstrap status tracking for requests that
	 * Hyperbridge hasn't seen.
	 */
	aggregateTransactionWithCommitment(
		commitment: HexString,
	): Promise<Awaited<ReturnType<SubstrateChain["submitUnsigned"]>>> {
		return this.postRequest.aggregateTransactionWithCommitment(commitment)
	}

	/**
	 * Snapshot query for a GET request: returns the indexed record with all
	 * inferred finality events (`SOURCE_FINALIZED`, `HYPERBRIDGE_FINALIZED`)
	 * sorted by progress weight.
	 */
	queryGetRequestWithStatus(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		return this.getRequest.queryGetRequestWithStatus(hash)
	}

	/**
	 * Streams status updates for a GET request. Ends when the response is
	 * delivered back to the source chain or the request's timeout elapses.
	 * Yields the `HYPERBRIDGE_FINALIZED` event with source-chain calldata for
	 * submitting the response (bundled `handleConsensus` + `handleGetResponses`
	 * via `batchCall`).
	 */
	getRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		return this.getRequest.getRequestStatusStream(hash)
	}


	/**
	 * Low-level watcher that yields a single `PENDING_TIMEOUT` event once
	 * `chain.timestamp()` passes `timeoutTimestamp`. Used internally by the
	 * status streams; exposed for callers that want to race their own logic
	 * against a request's deadline.
	 */
	timeoutStream(timeoutTimestamp: bigint, chain: IChain): AsyncGenerator<RequestStatusWithMetadata, void> {
		return timeoutStream(this.ctx, timeoutTimestamp, chain)
	}
}

interface PartialClientConfig extends Omit<ClientConfig, "pollInterval"> {
	pollInterval?: number
}
