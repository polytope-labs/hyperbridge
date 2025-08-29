import "log-timestamp"
import { describe, it, expect, beforeAll } from "vitest"
import { createQueryClient, _queryTokenPriceInternal } from "@/query-client"
import type { IndexerQueryClient } from "@/types"

describe.sequential("Token Price and Registry Integration Tests", () => {
	let queryClient: IndexerQueryClient

	beforeAll(async () => {
		queryClient = createQueryClient({ url: process.env.INDEXER_URL! })
	}, 10_000)

	it.sequential(
		"should query and validate token price indexing",
		async () => {
			try {
				const symbol = "DOT"
				const tokenPrice = await _queryTokenPriceInternal({ symbol, queryClient })

				expect(tokenPrice).toBeDefined()
				expect(tokenPrice!.symbol).toBe(symbol)
				expect(tokenPrice!.currency).toBe("USD")
				expect(tokenPrice!.lastUpdatedAt).toBeDefined()
				expect(parseFloat(tokenPrice!.price)).toBeGreaterThan(0)
			} catch (error) {
				console.error(error)
				expect(error).toBeUndefined()
			}
		},
		20_000,
	)

	it.sequential(
		"should validate token price updates and freshness",
		async () => {
			const tokenPricesQuery = `
			query RecentTokenPrices {
				tokenPrices(
					first: 5,
					orderBy: LAST_UPDATED_AT_DESC
				) {
					nodes {
						symbol
						price
						lastUpdatedAt
					}
				}
			}
		`

			const result = (await queryClient.request(tokenPricesQuery)) as {
				tokenPrices: { nodes: [{ symbol: string; price: string; lastUpdatedAt: bigint }] }
			}
			expect(result.tokenPrices.nodes).toBeInstanceOf(Array)
			expect(result.tokenPrices.nodes.length).toBeGreaterThan(0)

			const now = Date.now()
			for (const tokenPrice of result.tokenPrices.nodes) {
				expect(tokenPrice.symbol).toBeDefined()
				expect(parseFloat(tokenPrice.price)).toBeGreaterThan(0)

				const lastUpdated = Number(tokenPrice.lastUpdatedAt)
				const hoursSinceUpdate = (now - lastUpdated) / (1000 * 60 * 60)

				expect(lastUpdated).toBeGreaterThan(0)
				expect(hoursSinceUpdate).toBeLessThanOrEqual(1)
			}
		},
		15_000,
	)

	it.sequential(
		"should validate indexer connectivity and basic functionality",
		async () => {
			const tokens = `
			query TokenRegistries {
				tokenRegistries(first: 1) {
					totalCount
				}
			}
		`

			try {
				const result = (await queryClient.request(tokens)) as { tokenRegistries: { totalCount: number } }

				expect(result.tokenRegistries).toBeDefined()
				expect(typeof result.tokenRegistries.totalCount).toBe("number")
				expect(result.tokenRegistries.totalCount).toBeGreaterThan(0)
			} catch (error) {
				console.error(error)
				expect(error).toBeUndefined()
			}
		},
		10_000,
	)
})
