## Reward Points GraphQL API (Quick Guide)

Query reward points and activity logs indexed from the Hyperbridge protocol. Entities: `Rewards`, `RewardsActivityLog`.

### Schema (relevant fields)

- `Rewards`: `id`, `address`, `chain`, `points`, `earnerType`
- `RewardsActivityLog`: `id`, `chain`, `points`, `transactionHash`, `earnerAddress`, `earnerType`, `activityType`, `description`, `createdAt`

### Examples

1. Get a user's points across all chains and roles

```graphql
query GetUserPoints($address: String!) {
	rewards(filter: { address: { equalTo: $address } }, orderBy: POINTS_DESC) {
		nodes {
			id
			address
			chain
			earnerType
			points
		}
	}
}
```

2. Get leaderboard for a chain

```graphql
query ChainLeaderboard($chain: String!, $limit: Int = 50) {
	rewards(filter: { chain: { equalTo: $chain } }, orderBy: POINTS_DESC, first: $limit) {
		nodes {
			address
			earnerType
			points
		}
	}
}
```

3. Get entries by minimum points

```graphql
query ByPoints($min: BigInt!) {
	rewards(filter: { points: { greaterThanOrEqualTo: $min } }, orderBy: POINTS_DESC) {
		nodes {
			address
			chain
			earnerType
			points
		}
	}
}
```

4. Recent activity for a user

```graphql
query RecentActivity($address: String!, $limit: Int = 25) {
	rewardsActivityLogs(filter: { earnerAddress: { equalTo: $address } }, orderBy: CREATED_AT_DESC, first: $limit) {
		nodes {
			timestamp: createdAt
			activityType
			points
			transactionHash
			description
			chain
			earnerType
		}
	}
}
```

### cURL (sample)

```bash
curl -s -X POST -H "Content-Type: application/json" \
  -d '{
    "query": "query($address:String!){ rewards(filter:{address:{equalTo:$address}}, orderBy: POINTS_DESC){ nodes{ id address chain earnerType points } } }",
    "variables": { "address": "0xYourAddress" }
  }' \
  YOUR_GRAPHQL_ENDPOINT
```
