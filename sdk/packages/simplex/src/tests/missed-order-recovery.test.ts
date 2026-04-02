import { describe, it, expect, vi } from "vitest"


import { GraphQLClient } from "graphql-request"
import { orderCommitment } from "@hyperbridge/sdk"
import {
	transformIndexerOrderToSdkOrder,
	type IndexerOrderV2Node,
} from "@/services/MissedOrderRecoveryService"

const INDEXER_URL = "https://nexus.indexer.polytope.technology"

/**
 * Query to fetch recent IOrderV2 entries with all nested assets.
 * Uses CREATED_AT_DESC so we get the most recent orders.
 */
const FETCH_RECENT_ORDERS = `
query RecentOrders($first: Int!) {
  iOrderV2s(
    orderBy: CREATED_AT_DESC
    first: $first
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
  }
}
`

interface RecentOrdersResponse {
	iOrderV2s: {
		nodes: IndexerOrderV2Node[]
	}
}

describe("Missed Order Recovery — transformIndexerOrderToSdkOrder", () => {
	it("transforms 10 live indexer orders and commitment matches", async () => {
		const client = new GraphQLClient(INDEXER_URL)

		const response = await client.request<RecentOrdersResponse>(FETCH_RECENT_ORDERS, {
			first: 10,
		})

		const nodes = response.iOrderV2s.nodes
		expect(nodes.length).toBeGreaterThan(0)

		for (const node of nodes) {
			const order = transformIndexerOrderToSdkOrder(node)

			// The computed commitment must match what the indexer stored
			const computed = orderCommitment(order)
			expect(computed).toBe(node.commitment)

			// Basic structural checks
			expect(order.id).toBe(computed)
			expect(typeof order.deadline).toBe("bigint")
			expect(typeof order.nonce).toBe("bigint")
			expect(typeof order.fees).toBe("bigint")
			expect(order.inputs.length).toBe(node.inputAssets.nodes.length)
			expect(order.output.assets.length).toBe(node.outputAssets.nodes.length)
			expect(order.predispatch.assets.length).toBe(node.predispatchAssets.nodes.length)

			for (const input of order.inputs) {
				expect(typeof input.amount).toBe("bigint")
			}
			for (const output of order.output.assets) {
				expect(typeof output.amount).toBe("bigint")
			}
		}
	}, 30_000)
})
