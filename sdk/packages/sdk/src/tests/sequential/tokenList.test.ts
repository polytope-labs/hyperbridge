import "log-timestamp"
import { describe, it, expect, beforeAll } from "vitest"
import { createQueryClient } from "@/query-client"
import type { IndexerQueryClient } from "@/types"

const GRAPHQL_ENDPOINT = "http://localhost:3100/graphql"

describe.sequential("TokenList and SyncState Integration Tests", () => {
	let queryClient: IndexerQueryClient

	beforeAll(async () => {
		queryClient = createQueryClient({ url: GRAPHQL_ENDPOINT })
	}, 10_000)

	it.sequential(
		"should query token lists",
		async () => {
			const query = `
				query GetAllTokenLists {
					tokenLists(first: 10) {
						nodes {
							id
							tokenAddress
							chainId
							tokenName
							tokenSymbol
							pairedWith
						}
					}
				}
			`

			try {
				const result = (await queryClient.request(query)) as {
					tokenLists: {
						nodes: Array<{
							id: string
							tokenAddress: string
							chainId: string
							tokenName: string
							tokenSymbol: string
							pairedWith: string[]
						}>
					}
				}

				expect(result.tokenLists).toBeDefined()
				expect(result.tokenLists.nodes).toBeInstanceOf(Array)

				if (result.tokenLists.nodes.length > 0) {
					const token = result.tokenLists.nodes[0]
					expect(token.id).toBeDefined()
					expect(token.tokenAddress).toBeDefined()
					expect(token.chainId).toBeDefined()
					expect(token.tokenSymbol).toBeDefined()
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		20_000,
	)

	it.sequential(
		"should query token lists by chain ID",
		async () => {
			const chainId = "1"
			const query = `
				query GetTokenListsByChain($chainId: String!) {
					tokenLists(filter: { chainId: { equalTo: $chainId } }, first: 5) {
						nodes {
							id
							tokenAddress
							chainId
							tokenSymbol
						}
					}
				}
			`

			try {
				const result = (await queryClient.request(query, { chainId })) as {
					tokenLists: {
						nodes: Array<{
							id: string
							tokenAddress: string
							chainId: string
							tokenSymbol: string
						}>
					}
				}

				expect(result.tokenLists).toBeDefined()
				for (const token of result.tokenLists.nodes) {
					expect(token.chainId).toBe(chainId)
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		20_000,
	)

	it.sequential(
		"should query token list by ID",
		async () => {
			const listQuery = `
				query GetFirstTokenList {
					tokenLists(first: 1) {
						nodes {
							id
						}
					}
				}
			`

			try {
				const listResult = (await queryClient.request(listQuery)) as {
					tokenLists: {
						nodes: Array<{ id: string }>
					}
				}

				if (listResult.tokenLists.nodes.length === 0) {
					console.warn("No token lists found, skipping test")
					return
				}

				const tokenId = listResult.tokenLists.nodes[0].id
				const query = `
					query GetTokenListById($id: String!) {
						tokenList(id: $id) {
							id
							tokenAddress
							chainId
							tokenSymbol
						}
					}
				`

				const result = (await queryClient.request(query, { id: tokenId })) as {
					tokenList: {
						id: string
						tokenAddress: string
						chainId: string
						tokenSymbol: string
					} | null
				}

				expect(result.tokenList).toBeDefined()
				if (result.tokenList) {
					expect(result.tokenList.id).toBe(tokenId)
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		20_000,
	)

	it.sequential(
		"should query sync states",
		async () => {
			const query = `
				query GetAllSyncStates {
					tokenListSyncStates {
						nodes {
							id
							networkName
							chainId
							currentPage
						}
					}
				}
			`

			try {
				const result = (await queryClient.request(query)) as {
					tokenListSyncStates: {
						nodes: Array<{
							id: string
							networkName: string
							chainId: string
							currentPage: number
						}>
					}
				}

				expect(result.tokenListSyncStates).toBeDefined()
				expect(result.tokenListSyncStates.nodes).toBeInstanceOf(Array)

				if (result.tokenListSyncStates.nodes.length > 0) {
					const syncState = result.tokenListSyncStates.nodes[0]
					expect(syncState.id).toBeDefined()
					expect(syncState.networkName).toBeDefined()
					expect(syncState.chainId).toBeDefined()
					expect(syncState.currentPage).toBeGreaterThanOrEqual(1)
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		20_000,
	)

	it.sequential(
		"should query sync state by network name",
		async () => {
			const networkName = "eth"
			const query = `
				query GetSyncStateByNetwork($networkName: String!) {
					tokenListSyncState(id: $networkName) {
						id
						networkName
						chainId
						currentPage
					}
				}
			`

			try {
				const result = (await queryClient.request(query, { networkName })) as {
					tokenListSyncState: {
						id: string
						networkName: string
						chainId: string
						currentPage: number
					} | null
				}

				if (result.tokenListSyncState) {
					expect(result.tokenListSyncState.id).toBe(networkName)
					expect(result.tokenListSyncState.currentPage).toBeGreaterThanOrEqual(1)
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		20_000,
	)
})
