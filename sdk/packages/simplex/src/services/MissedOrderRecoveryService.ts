import { GraphQLClient } from "graphql-request"
import { toHex } from "viem"
import { type Order, type HexString, orderCommitment, bytes20ToBytes32, hexToString } from "@hyperbridge/sdk"
import type { FillerStrategy } from "@/strategies/base"
import type { LimitOrderStorageService } from "./LimitOrderStorageService"
import type { ChainClientManager } from "./ChainClientManager"
import { getLogger } from "./Logger"

// ── Indexer response types ───────────────────────────────────────────────────

export interface IndexerAssetNode {
	token: string
	amount: string
	index: number
}

export interface IndexerOutputAssetNode extends IndexerAssetNode {
	beneficiary: string
}

export interface IndexerOrderV2Node {
	id: string
	user: string
	sourceChain: string
	destChain: string
	commitment: string
	deadline: string
	nonce: string
	fees: string
	session: string | null
	predispatchCalldata: string
	postDispatchCalldata: string
	inputAssets: { nodes: IndexerAssetNode[] }
	outputAssets: { nodes: IndexerOutputAssetNode[] }
	predispatchAssets: { nodes: IndexerAssetNode[] }
	createdAt: string
	blockNumber: string
	blockTimestamp: string
	transactionHash: string
}

interface MissedOrdersResponse {
	iOrderV2s: {
		nodes: IndexerOrderV2Node[]
		pageInfo: {
			hasNextPage: boolean
			endCursor: string
		}
	}
}

// ── GraphQL query ────────────────────────────────────────────────────────────

const MISSED_ORDERS_QUERY = `
query MissedOrders($destChains: [String!]!, $createdAfter: Datetime!, $first: Int!, $after: String) {
  iOrderV2s(
    filter: {
      destChain: { in: $destChains }
      status: { equalTo: PLACED }
      createdAt: { greaterThan: $createdAfter }
    }
    orderBy: CREATED_AT_ASC
    first: $first
    after: $after
  ) {
    nodes {
      id
      user
      sourceChain
      destChain
      commitment
      deadline
      nonce
      fees
      session
      predispatchCalldata
      postDispatchCalldata
      inputAssets(orderBy: INDEX_ASC) {
        nodes { token amount index }
      }
      outputAssets(orderBy: INDEX_ASC) {
        nodes { token amount index beneficiary }
      }
      predispatchAssets(orderBy: INDEX_ASC) {
        nodes { token amount index }
      }
      createdAt
      blockNumber
      blockTimestamp
      transactionHash
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
`

// ── Transform ────────────────────────────────────────────────────────────────

/**
 * Transforms an indexer `IOrderV2` GraphQL node into the SDK `Order` type.
 *
 * This is a pure function, exported for unit testing. After constructing the
 * order, it computes `orderCommitment()` and assigns it to `order.id`.
 */
export function transformIndexerOrderToSdkOrder(node: IndexerOrderV2Node): Order {
	// The indexer stores chain IDs as hex-encoded UTF-8 (e.g. "0x45564d2d38343533"
	// for "EVM-8453"). Convert to plain strings to match the format used by
	// EventMonitor and FXFiller's token1 map. The orderCommitment() function
	// handles re-encoding to hex internally via transformOrderForContract().
	const source = node.sourceChain.startsWith("0x") ? hexToString(node.sourceChain) : node.sourceChain
	const destination = node.destChain.startsWith("0x") ? hexToString(node.destChain) : node.destChain

	const order: Order = {
		// The indexer stores `user` as a 20-byte address; the on-chain struct uses bytes32.
		user: bytes20ToBytes32(node.user) as HexString,
		source: source as HexString,
		destination: destination as HexString,
		deadline: BigInt(node.deadline),
		nonce: BigInt(node.nonce),
		fees: BigInt(node.fees),
		session: (node.session ?? "0x0000000000000000000000000000000000000000") as HexString,
		predispatch: {
			assets: node.predispatchAssets.nodes.map((a) => ({
				token: a.token as HexString,
				amount: BigInt(a.amount),
			})),
			call: (node.predispatchCalldata || "0x") as HexString,
		},
		inputs: node.inputAssets.nodes.map((a) => ({
			token: a.token as HexString,
			amount: BigInt(a.amount),
		})),
		output: {
			beneficiary: bytes20ToBytes32(
				node.outputAssets.nodes[0]?.beneficiary ?? "0x0000000000000000000000000000000000000000",
			) as HexString,
			assets: node.outputAssets.nodes.map((a) => ({
				token: a.token as HexString,
				amount: BigInt(a.amount),
			})),
			call: (node.postDispatchCalldata || "0x") as HexString,
		},
	}

	order.id = orderCommitment(order)
	return order
}

// ── Service ──────────────────────────────────────────────────────────────────

const BATCH_SIZE = 1000
const MAX_ORDERS = 5000

/**
 * Queries the Hyperbridge indexer at startup for orders placed while the filler
 * was offline, filters them through FXFiller strategies, and stores valid ones
 * as limit orders for the existing sweep to re-evaluate.
 *
 * Runs once during `IntentFiller.initialize()`.
 */
export class MissedOrderRecoveryService {
	private client: GraphQLClient
	private limitOrderStorage: LimitOrderStorageService
	private strategies: FillerStrategy[]
	private chainClientManager: ChainClientManager
	private logger = getLogger("missed-order-recovery")

	constructor(
		indexerUrl: string,
		limitOrderStorage: LimitOrderStorageService,
		strategies: FillerStrategy[],
		chainClientManager: ChainClientManager,
	) {
		this.client = new GraphQLClient(indexerUrl)
		this.limitOrderStorage = limitOrderStorage
		this.strategies = strategies
		this.chainClientManager = chainClientManager
	}

	/**
	 * Queries the indexer for unfilled orders created since the last shutdown,
	 * filters out expired and unfillable orders, and stores the rest as limit orders.
	 *
	 * @returns The number of orders stored.
	 */
	async recover(): Promise<number> {
		const lastShutdown = this.limitOrderStorage.getLastShutdownTime()
		if (!lastShutdown) {
			this.logger.info("No previous shutdown recorded, skipping missed order recovery")
			return 0
		}

		// Collect destination chains from all FXFiller strategies.
		// Keep the plain IDs (e.g. "EVM-8453") for RPC calls, and convert to
		// hex-encoded UTF-8 (e.g. "0x45564d2d38343533") for the indexer query
		// since the indexer stores chain IDs in their on-chain hex form.
		const plainDestChains = new Set<string>()
		for (const strategy of this.strategies) {
			if (strategy.name === "FXFiller" && typeof (strategy as any).getDestinationChains === "function") {
				for (const chain of (strategy as any).getDestinationChains() as string[]) {
					plainDestChains.add(chain)
				}
			}
		}

		if (plainDestChains.size === 0) {
			this.logger.info("No FXFiller destination chains configured, skipping recovery")
			return 0
		}

		const hexDestChains = [...plainDestChains].map((c) => (c.startsWith("0x") ? c : toHex(c)))

		this.logger.info(
			{ lastShutdown, destChains: [...plainDestChains] },
			"Starting missed order recovery from indexer",
		)

		// Fetch orders in batches of 1000, up to 5000 total
		const allNodes: IndexerOrderV2Node[] = []
		let cursor: string | undefined
		let fetched = 0

		while (fetched < MAX_ORDERS) {
			const batchSize = Math.min(BATCH_SIZE, MAX_ORDERS - fetched)

			const response = await this.client.request<MissedOrdersResponse>(MISSED_ORDERS_QUERY, {
				destChains: hexDestChains,
				createdAfter: lastShutdown,
				first: batchSize,
				after: cursor ?? null,
			})

			const nodes = response.iOrderV2s.nodes
			allNodes.push(...nodes)
			fetched += nodes.length

			this.logger.debug({ batch: Math.ceil(fetched / BATCH_SIZE), fetched }, "Fetched order batch")

			if (!response.iOrderV2s.pageInfo.hasNextPage || nodes.length < batchSize) {
				break
			}

			cursor = response.iOrderV2s.pageInfo.endCursor
		}

		if (allNodes.length === 0) {
			this.logger.info("No missed orders found in indexer")
			return 0
		}

		this.logger.info({ total: allNodes.length }, "Fetched missed orders from indexer, filtering")

		// Batch block number queries per destination chain for deadline checks.
		// Key by hex chain ID (as stored by the indexer) so we can look up per-node.
		const blockNumberCache = new Map<string, bigint>()
		for (const chain of plainDestChains) {
			try {
				const client = this.chainClientManager.getPublicClient(chain)
				const blockNumber = await client.getBlockNumber()
				blockNumberCache.set(chain.startsWith("0x") ? chain : toHex(chain), blockNumber)
			} catch (err) {
				this.logger.warn({ chain, err }, "Failed to get block number for chain, will skip deadline check")
			}
		}

		let stored = 0

		for (const node of allNodes) {
			try {
				const order = transformIndexerOrderToSdkOrder(node)

				// Check deadline
				const currentBlock = blockNumberCache.get(node.destChain)
				if (currentBlock !== undefined && currentBlock >= order.deadline) {
					continue
				}

				// Check if any FXFiller strategy can fill this order
				let canFill = false
				let matchedStrategy: FillerStrategy | undefined
				for (const strategy of this.strategies) {
					if (strategy.name !== "FXFiller") continue
					try {
						if (await strategy.canFill(order)) {
							canFill = true
							matchedStrategy = strategy
							break
						}
					} catch (err) {
						this.logger.debug({ orderId: order.id, err }, "canFill check failed")
					}
				}

				if (!canFill || !matchedStrategy) {
					continue
				}

				this.limitOrderStorage.storeLimitOrder(order, matchedStrategy.name)
				stored++
			} catch (err) {
				this.logger.warn({ commitment: node.commitment, err }, "Failed to process missed order")
			}
		}

		this.logger.info({ fetched: allNodes.length, stored }, "Missed order recovery complete")
		return stored
	}
}
