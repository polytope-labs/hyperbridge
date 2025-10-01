import fetch from "node-fetch"

const GRAPHQL_ENDPOINT = "http://localhost:3100/graphql"

interface HyperbridgeRelayerReward {
	id: string
	totalRewardAmount: string
	totalConsensusRewardAmount: string
	totalMessagingRewardAmount: string
	reputationAssetBalance: string
}

describe("Incentives GraphQL Test", () => {
	it("should fetch a list of HyperbridgeRelayerReward entities", async () => {
		const query = `
         query {
            hyperbridgeRelayerRewards(first: 5, orderBy: TOTAL_REWARD_AMOUNT_DESC) {
               nodes {
                  id
                  totalRewardAmount
                  totalConsensusRewardAmount
                  totalMessagingRewardAmount
                  reputationAssetBalance
               }
            }
         }
      `
		const startTime = Date.now()
		const timeout = 240000
		const pollInterval = 2000

		let rewardNodes: HyperbridgeRelayerReward[] | null = null

		while (Date.now() - startTime < timeout) {
			const response = await fetch(GRAPHQL_ENDPOINT, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ query }),
			})

			if (!response.ok) {
				console.error(`Received non-ok response: ${response.status}`)
				await new Promise(resolve => setTimeout(resolve, pollInterval))
				continue
			}

			const json = (await response.json()) as any

			if (json.errors) {
				throw new Error(
					`GraphQL query failed: ${JSON.stringify(json.errors)}`,
				)
			}

			if (
				json.data?.hyperbridgeRelayerRewards?.nodes &&
				json.data.hyperbridgeRelayerRewards.nodes.length > 0
			) {
				rewardNodes = json.data.hyperbridgeRelayerRewards.nodes
				break
			}

			await new Promise(resolve => setTimeout(resolve, pollInterval))
		}

		if (!rewardNodes) {
			throw new Error(
				`Test timed out after ${
					timeout / 1000
				}s waiting for HyperbridgeRelayerReward entities to be created.`,
			)
		}

		console.log(
			`Successfully fetched ${rewardNodes.length} relayer rewards.`,
		)
		expect(rewardNodes.length).toBeGreaterThan(0)

		const firstReward = rewardNodes[0]
		expect(firstReward.id).toBeDefined()
		expect(firstReward.totalRewardAmount).toBeDefined()
	}, 30000)
})