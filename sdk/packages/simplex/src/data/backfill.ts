import type { Logger } from "@/services/Logger"
import { referrerFrom, toBytes20, type TokenDescriber } from "./recorder"
import type { ActivityEvent, ActivityStore, OrderLeg, OrderSummary } from "./types"

/** Public Hyperbridge indexers, one per network. */
export const DEFAULT_INDEXER_URLS = {
	mainnet: "https://nexus.indexer.polytope.technology/",
	testnet: "https://gargantua.indexer.polytope.technology/",
} as const

/** Only the fields the summary needs; the entity carries much more. */
const ORDER_QUERY = `
query OrderSummary($commitment: String!) {
  iOrderV3s(filter: { commitment: { equalTo: $commitment } }) {
    nodes {
      user
      sourceChain
      destChain
      deadline
      referrer
      transactionHash
      inputAssets { nodes { token amount index } }
      outputAssets { nodes { token amount index } }
      statusMetadata { nodes { status chain transactionHash filler } }
    }
  }
}`

interface IndexedAsset {
	token: string
	amount: string
	index: number
}

interface IndexedStatus {
	status: string
	/** Chain id as a decimal string. */
	chain: string
	transactionHash: string
	filler: string | null
}

interface IndexedOrder {
	user: string
	sourceChain: string
	destChain: string
	deadline: string
	referrer: string | null
	transactionHash: string
	inputAssets: { nodes: IndexedAsset[] }
	outputAssets: { nodes: IndexedAsset[] }
	statusMetadata: { nodes: IndexedStatus[] }
}

export interface BackfillOptions {
	store: ActivityStore
	indexerUrl: string
	/** This filler's address; when given, orders it bid on are settled from the indexer's fill record. */
	fillerAddress?: string
	describeToken?: TokenDescriber
	/** Receives every row that gained a summary, so live consumers can refresh it. */
	onUpdated?: (rows: ActivityEvent[]) => void
	logger: Logger
	/** Distinct orders to look up per run. */
	limit?: number
	/** Parallel indexer requests. */
	concurrency?: number
	fetchImpl?: typeof fetch
}

/**
 * Fills in order summaries on activity rows recorded before the filler captured
 * them, from the Hyperbridge indexer. Best effort: an order the indexer does
 * not know stays as it is, and any failure ends the run with a log line rather
 * than an error — the feed is observability.
 */
export async function backfillOrderSummaries(
	options: BackfillOptions,
): Promise<{ orders: number; rows: number; settled: number }> {
	const { store, indexerUrl, fillerAddress, describeToken, onUpdated, logger, limit = 500, concurrency = 4 } = options
	const fetchImpl = options.fetchImpl ?? fetch
	const totals = { orders: 0, rows: 0, settled: 0 }

	const run = async (ids: string[], handle: (id: string, indexed: IndexedOrder) => Promise<void>) => {
		const queue = [...ids]
		const worker = async () => {
			for (let id = queue.shift(); id !== undefined; id = queue.shift()) {
				try {
					const indexed = await fetchOrder(fetchImpl, indexerUrl, id)
					if (!indexed) {
						logger.debug({ orderId: id }, "Indexer has no record of this order; leaving the rows as they are")
						continue
					}
					await handle(id, indexed)
				} catch (err) {
					logger.warn({ err, orderId: id }, "Activity backfill failed for an order")
				}
			}
		}
		await Promise.all(Array.from({ length: Math.min(concurrency, ids.length) }, worker))
	}

	// Pass 1: order details for rows recorded before they were captured.
	let missing: string[] = []
	try {
		missing = await store.orderIdsMissingSummary(limit)
	} catch (err) {
		logger.warn({ err }, "Activity backfill could not list rows without order details")
	}
	if (missing.length > 0) {
		logger.info({ orders: missing.length, indexerUrl }, "Backfilling order details on the activity feed")
		await run(missing, async (id, indexed) => {
			const summary = await toSummary(indexed, describeToken, logger)
			const rows = await store.attachOrder(id, summary)
			if (rows.length > 0) {
				totals.orders += 1
				totals.rows += rows.length
				onUpdated?.(rows)
			}
		})
	}

	// Pass 2: outcomes for orders this filler bid on but never saw settle — a
	// fill that landed while it was down, or a bid-time "filled" row from before
	// bids and fills were told apart.
	if (fillerAddress) {
		let unsettled: string[] = []
		try {
			unsettled = await store.unsettledOrders(limit)
		} catch (err) {
			logger.warn({ err }, "Activity backfill could not list unsettled bids")
		}
		if (unsettled.length > 0) {
			logger.info({ orders: unsettled.length }, "Settling bid outcomes from the indexer")
			const ours = fillerAddress.toLowerCase()
			await run(unsettled, async (id, indexed) => {
				const fill = indexed.statusMetadata.nodes.find((node) => node.status === "FILLED")
				if (!fill) return // still open on chain
				const retyped = await store.retypeLegacyBid(id)
				const won = (fill.filler ?? "").toLowerCase() === ours
				const summary =
					retyped.find((row) => row.order)?.order ?? (await toSummary(indexed, describeToken, logger))
				const chainId = Number(fill.chain)
				const settled = await store.record({
					type: won ? "filled" : "lost",
					orderId: id,
					chainId: Number.isFinite(chainId) ? chainId : null,
					txHash: fill.transactionHash || null,
					reason: won ? null : (fill.filler ?? null),
					order: summary,
				})
				totals.settled += 1
				onUpdated?.([...retyped, settled])
			})
		}
	}

	logger.info(totals, "Activity backfill finished")
	return totals
}

async function fetchOrder(fetchImpl: typeof fetch, indexerUrl: string, commitment: string): Promise<IndexedOrder | null> {
	const response = await fetchImpl(indexerUrl, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ query: ORDER_QUERY, variables: { commitment } }),
		signal: AbortSignal.timeout(15_000),
	})
	if (!response.ok) throw new Error(`Indexer responded ${response.status}`)
	const body = (await response.json()) as {
		data?: { iOrderV3s?: { nodes: IndexedOrder[] } }
		errors?: Array<{ message: string }>
	}
	if (body.errors?.length) throw new Error(body.errors.map((e) => e.message).join("; "))
	return body.data?.iOrderV3s?.nodes[0] ?? null
}

async function toSummary(
	indexed: IndexedOrder,
	describeToken: TokenDescriber | undefined,
	logger: Logger,
): Promise<OrderSummary> {
	const legs = (chain: string, assets: IndexedAsset[]): Promise<OrderLeg[]> =>
		Promise.all(
			[...assets]
				.sort((a, b) => a.index - b.index)
				.map(async ({ token, amount }) => {
					const address = toBytes20(token)
					let described = { symbol: null as string | null, decimals: null as number | null }
					if (describeToken) {
						try {
							described = await describeToken(chain, address)
						} catch (err) {
							logger.debug({ err, chain, token: address }, "Token metadata lookup failed")
						}
					}
					return { token: address, amount, ...described }
				}),
		)
	const user = toBytes20(indexed.user)
	const [inputs, outputs] = await Promise.all([
		legs(indexed.sourceChain, indexed.inputAssets.nodes),
		legs(indexed.destChain, indexed.outputAssets.nodes),
	])
	return {
		user,
		source: indexed.sourceChain,
		destination: indexed.destChain,
		placedTxHash: indexed.transactionHash || null,
		referrer: referrerFrom(indexed.referrer ?? undefined, user),
		inputs,
		outputs,
		deadline: indexed.deadline,
	}
}
