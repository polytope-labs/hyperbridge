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

	/**
	 * Helper function to poll until token list has entries
	 * @param maxWaitMs Maximum time to wait in milliseconds (default 5 minutes)
	 * @param pollIntervalMs Interval between polls in milliseconds (default 5 seconds)
	 */
	const waitForTokenList = async (maxWaitMs = 300_000, pollIntervalMs = 5_000): Promise<boolean> => {
		const startTime = Date.now()
		const query = `
			query GetTokenListCount {
				tokenLists(first: 1) {
					totalCount
					nodes {
						id
					}
				}
			}
		`

		while (Date.now() - startTime < maxWaitMs) {
			try {
				const result = (await queryClient.request(query)) as {
					tokenLists: {
						totalCount: number
						nodes: Array<{ id: string }>
					}
				}

				if (result.tokenLists.totalCount > 0 && result.tokenLists.nodes.length > 0) {
					console.log(`Token list populated with ${result.tokenLists.totalCount} tokens`)
					return true
				}

				const elapsed = Math.floor((Date.now() - startTime) / 1000)
				console.log(`Waiting for token list to be populated... (${elapsed}s elapsed)`)
				await new Promise((resolve) => setTimeout(resolve, pollIntervalMs))
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running")
					return false
				}
				// Continue polling on other errors
				await new Promise((resolve) => setTimeout(resolve, pollIntervalMs))
			}
		}

		throw new Error(`Timeout waiting for token list after ${maxWaitMs / 1000}s`)
	}

	/**
	 * Helper function to poll until sync state has entries
	 * @param maxWaitMs Maximum time to wait in milliseconds (default 5 minutes)
	 * @param pollIntervalMs Interval between polls in milliseconds (default 5 seconds)
	 */
	const waitForSyncState = async (maxWaitMs = 300_000, pollIntervalMs = 5_000): Promise<boolean> => {
		const startTime = Date.now()
		const query = `
			query GetSyncStateCount {
				tokenListSyncStates(first: 1) {
					totalCount
					nodes {
						id
					}
				}
			}
		`

		while (Date.now() - startTime < maxWaitMs) {
			try {
				const result = (await queryClient.request(query)) as {
					tokenListSyncStates: {
						totalCount: number
						nodes: Array<{ id: string }>
					}
				}

				if (result.tokenListSyncStates.totalCount > 0 && result.tokenListSyncStates.nodes.length > 0) {
					console.log(`Sync state populated with ${result.tokenListSyncStates.totalCount} entries`)
					return true
				}

				const elapsed = Math.floor((Date.now() - startTime) / 1000)
				console.log(`Waiting for sync state to be populated... (${elapsed}s elapsed)`)
				await new Promise((resolve) => setTimeout(resolve, pollIntervalMs))
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running")
					return false
				}
				// Continue polling on other errors
				await new Promise((resolve) => setTimeout(resolve, pollIntervalMs))
			}
		}

		throw new Error(`Timeout waiting for sync state after ${maxWaitMs / 1000}s`)
	}

	it.sequential(
		"should query token lists",
		async () => {
			// Wait for token list to be populated
			const hasTokens = await waitForTokenList()
			if (!hasTokens) {
				console.warn("Token list not populated within timeout, skipping test")
				return
			}

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
					console.log(`Pairs: ${token.pairedWith}`)
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("ECONNREFUSED")) {
					console.warn("Indexer not running, skipping test")
					return
				}
				throw error
			}
		},
		300_000, // 5 minutes
	)

	it.sequential(
		"should query token lists by chain ID",
		async () => {
			// Wait for token list to be populated
			const hasTokens = await waitForTokenList()
			if (!hasTokens) {
				console.warn("Token list not populated within timeout, skipping test")
				return
			}

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
		300_000, // 5 minutes
	)

	it.sequential(
		"should query token list by ID",
		async () => {
			// Wait for token list to be populated
			const hasTokens = await waitForTokenList()
			if (!hasTokens) {
				console.warn("Token list not populated within timeout, skipping test")
				return
			}

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
		300_000, // 5 minutes
	)

	it.sequential(
		"should query sync states",
		async () => {
			// Wait for sync state to be populated
			const hasSyncState = await waitForSyncState()
			if (!hasSyncState) {
				console.warn("Sync state not populated within timeout, skipping test")
				return
			}

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
		300_000, // 5 minutes
	)

	it.sequential(
		"should query sync state by network name",
		async () => {
			// Wait for sync state to be populated
			const hasSyncState = await waitForSyncState()
			if (!hasSyncState) {
				console.warn("Sync state not populated within timeout, skipping test")
				return
			}

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
		300_000, // 5 minutes
	)
})
