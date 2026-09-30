// The HyperFX orderbook serves every endpoint the indexer reads — the solver watchlist and the
// rates GraphQL API — on one host, so one variable configures them all.
import { ENV_CONFIG } from "@/constants"

/** The orderbook's base URL. Every endpoint below is resolved against it. */
export const ORDERBOOK_URL_VAR = "HYPERFX_ORDERBOOK_URL"

/** The solver watchlist: the solvers the orderbook wants inventory tracked for. */
export const WATCHLIST_PATH = "solvers"
/** The rates API, which prices tokens without a $1 peg. */
export const GRAPHQL_PATH = "graphql"

const ENDPOINT_PATHS = [WATCHLIST_PATH, GRAPHQL_PATH]

/**
 * `path` resolved against the configured orderbook, or undefined when none is configured or the
 * configured value is not a URL.
 *
 * A base naming an endpoint already is accepted as the host it is on: this replaced
 * `HYPERFX_WATCHLIST_URL`, which held the whole `.../solvers` URL, so that is the value an operator
 * is most likely to carry over. A base with any other path keeps it, so the orderbook can be served
 * under a prefix.
 */
export function orderbookEndpoint(path: string): string | undefined {
	const configured = (ENV_CONFIG as Record<string, string | null | undefined>)[ORDERBOOK_URL_VAR]
	if (!configured) return undefined

	let base: URL
	try {
		base = new URL(configured)
	} catch {
		logger.warn(`[orderbook] ${ORDERBOOK_URL_VAR} is not a URL: ${configured}`)
		return undefined
	}

	const segments = base.pathname.split("/").filter(Boolean)
	if (segments.length > 0 && ENDPOINT_PATHS.includes(segments[segments.length - 1])) segments.pop()
	// The trailing slash makes the base a directory, so a prefix is kept rather than replaced.
	base.pathname = segments.length > 0 ? `/${segments.join("/")}/` : "/"

	return new URL(path, base).toString()
}
