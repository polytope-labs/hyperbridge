import fetch from "node-fetch"

const TREASURY_ADDRESS = "13UVJyLkyUpEiXBx5p776dHQoBuuk3Y5PYp5Aa89rYWePWA3"
const GRAPHQL_ENDPOINT = "http://localhost:3100/graphql"

describe("Incentives GraphQL Test", () => {
	it("should fetch the single Treasury entity", async () => {
		const query = `
         query {
            treasury(id: "${TREASURY_ADDRESS}") {
               id
               totalAmountTransferredIn
               totalAmountTransferredOut
               totalBalance
               lastUpdatedAt
            }
         }
      `

		const startTime = Date.now()
		const timeout = 240000
		const pollInterval = 2000

		let treasuryEntity = null

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

			if (json.data && json.data.treasury) {
				treasuryEntity = json.data.treasury
				break
			}

			await new Promise(resolve => setTimeout(resolve, pollInterval))
		}

		if (!treasuryEntity) {
			throw new Error(
				`Test timed out after ${timeout / 1000}s waiting for the Treasury entity.`,
			)
		}

		console.log("Successfully fetched the Treasury entity.")
		// @ts-ignore
		expect(treasuryEntity.id).toEqual(TREASURY_ADDRESS)
		// @ts-ignore
		expect(treasuryEntity.totalBalance).toBeDefined()
	}, 30000)
})