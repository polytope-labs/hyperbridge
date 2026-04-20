import { LogLevels, createConsola } from "consola"

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

import { GetRequestStatus } from "./GetRequestStatus"
import { PostRequestStatus } from "./PostRequestStatus"
import { ProofFinalizer } from "./ProofFinalizer"
import { StateMachineQueries } from "./StateMachineQueries"
import { TimeoutFlow } from "./TimeoutFlow"
import type { ClientContext } from "./types"

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
	protected readonly queries: StateMachineQueries
	/** Builds HYPERBRIDGE_FINALIZED events (HandlerV1 + HandlerV2 batch paths). */
	protected readonly proofFinalizer: ProofFinalizer
	/** POST request snapshot + streaming status. */
	protected readonly postRequest: PostRequestStatus
	/** GET request snapshot + streaming status. */
	protected readonly getRequest: GetRequestStatus
	/** Timeout flow: pending → destination-finalized → hyperbridge-finalized → timed-out. */
	protected readonly timeout: TimeoutFlow

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

		this.queries = new StateMachineQueries(this.ctx)
		this.proofFinalizer = new ProofFinalizer(this.ctx, this.queries)
		this.timeout = new TimeoutFlow(this.ctx, this.queries)
		this.postRequest = new PostRequestStatus(
			this.ctx,
			this.queries,
			this.proofFinalizer,
			(ts, chain) => this.timeout.timeoutStream(ts, chain),
			(request) => this.timeout.addTimeoutFinalityEvents(request),
		)
		this.getRequest = new GetRequestStatus(this.ctx, this.queries, this.proofFinalizer, (ts, chain) =>
			this.timeout.timeoutStream(ts, chain),
		)
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

	// ── State machine queries ────────────────────────────────────────────

	queryStateMachineUpdateByHeight(args: {
		statemachineId: string
		chain: string
		height: number
	}): Promise<StateMachineUpdate | undefined> {
		return this.queries.queryStateMachineUpdateByHeight(args)
	}

	queryStateMachineUpdateByTimestamp(args: {
		statemachineId: string
		commitmentTimestamp: bigint
		chain: string
	}): Promise<StateMachineUpdate | undefined> {
		return this.queries.queryStateMachineUpdateByTimestamp(args)
	}

	queryLatestStateMachineHeight(args: { statemachineId: string; chain: string }): Promise<bigint | undefined> {
		return this.queries.queryLatestStateMachineHeight(args)
	}

	queryPostRequest(commitmentHash: HexString): Promise<PostRequestWithStatus | undefined> {
		return this.queries.queryPostRequest(commitmentHash)
	}

	queryGetRequest(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		return this.queries.queryGetRequest(hash)
	}

	queryResponseByRequestId(requestId: string) {
		return this.queries.queryResponseByRequestId(requestId)
	}

	// ── POST request flow ────────────────────────────────────────────────

	queryRequestWithStatus(hash: HexString): Promise<PostRequestWithStatus | undefined> {
		return this.postRequest.queryRequestWithStatus(hash)
	}

	postRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		return this.postRequest.postRequestStatusStream(hash)
	}

	// ── GET request flow ─────────────────────────────────────────────────

	queryGetRequestWithStatus(hash: HexString): Promise<GetRequestWithStatus | undefined> {
		return this.getRequest.queryGetRequestWithStatus(hash)
	}

	getRequestStatusStream(hash: HexString): AsyncGenerator<RequestStatusWithMetadata, void> {
		return this.getRequest.getRequestStatusStream(hash)
	}

	// ── Timeout flow ─────────────────────────────────────────────────────

	timeoutStream(timeoutTimestamp: bigint, chain: IChain): AsyncGenerator<RequestStatusWithMetadata, void> {
		return this.timeout.timeoutStream(timeoutTimestamp, chain)
	}

	postRequestTimeoutStream(hash: HexString): AsyncGenerator<PostRequestTimeoutStatus, void> {
		return this.timeout.postRequestTimeoutStream(hash)
	}

	aggregateTransactionWithCommitment(
		commitment: HexString,
	): Promise<Awaited<ReturnType<SubstrateChain["submitUnsigned"]>>> {
		return this.timeout.aggregateTransactionWithCommitment(commitment)
	}
}

interface PartialClientConfig extends Omit<ClientConfig, "pollInterval"> {
	pollInterval?: number
}
