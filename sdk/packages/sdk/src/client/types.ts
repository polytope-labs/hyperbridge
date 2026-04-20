import type { ConsolaInstance } from "consola"

import type { ClientConfig, IndexerQueryClient, RetryConfig } from "@/types"

/**
 * Shared dependencies passed to every sub-module of {@link IsmpClient}.
 *
 * Sub-modules read their chain configuration, GraphQL client, logger, and
 * retry helpers off of this object rather than reaching back into the facade.
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
