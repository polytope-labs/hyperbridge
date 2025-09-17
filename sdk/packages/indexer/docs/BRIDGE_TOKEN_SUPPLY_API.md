# BridgeTokenSupply GraphQL API Documentation

The BridgeTokenSupply API provides real-time access to Hyperbridge token supply data, including total supply and circulating supply metrics.

## Schema Definition

The `BridgeTokenSupply` entity tracks Hyperbridge token supply information with the following fields:

- `id`: Unique identifier (always "hyperbridge-token-supply")
- `totalSupply`: Total supply of Hyperbridge tokens (BigInt, indexed)
- `circulatingSupply`: Circulating supply excluding locked/staked amounts (BigInt, indexed)
- `lastUpdatedAt`: Timestamp of last update (BigInt, indexed)
- `createdAt`: Record creation timestamp (Date, indexed)

## GraphQL Query Examples

### 1. Get Current Token Supply

Fetch the current Hyperbridge token supply data:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetTokenSupply { bridgeTokenSupplies { id totalSupply circulatingSupply lastUpdatedAt createdAt } }"
  }' \
  YOUR_GRAPHQL_ENDPOINT
```

**GraphQL Query:**
```graphql
query GetTokenSupply {
  bridgeTokenSupplies {
    id
    totalSupply
    circulatingSupply
    lastUpdatedAt
    createdAt
  }
}
```

### 2. Get Token Supply by ID

Since there's only one token supply entity, you can fetch it by its known ID:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetTokenSupplyById($id: String!) { bridgeTokenSupply(id: $id) { id totalSupply circulatingSupply lastUpdatedAt createdAt } }",
    "variables": { "id": "hyperbridge-token-supply" }
  }' \
  YOUR_GRAPHQL_ENDPOINT
```

**GraphQL Query:**
```graphql
query GetTokenSupplyById($id: String!) {
  bridgeTokenSupply(id: $id) {
    id
    totalSupply
    circulatingSupply
    lastUpdatedAt
    createdAt
  }
}
```

## Example Response

Here's an example response from the BridgeTokenSupply API:

```json
{
  "data": {
    "bridgeTokenSupplies": [
      {
        "id": "hyperbridge-token-supply",
        "totalSupply": "1000000000000",
        "circulatingSupply": "750000000000",
        "lastUpdatedAt": "1734451200000",
        "createdAt": "2024-12-17T15:20:00.000Z"
      }
    ]
  }
}
```

## Field Explanations

- **totalSupply**: The complete token supply including all minted tokens (in token units, scaled down by 10^12 from raw blockchain units)
- **circulatingSupply**: Total supply minus locked/staked amounts and inactive issuance (in token units)
- **lastUpdatedAt**: Unix timestamp in milliseconds when the supply data was last updated
- **createdAt**: ISO 8601 timestamp when the record was first created

## Data Update Mechanism

The BridgeTokenSupply data is updated automatically by the indexer when:
- New blocks are processed on the Hyperbridge chain
- Token supply changes occur (minting, burning, locking, unlocking)
- The circulating supply calculation needs updating

The service fetches data directly from the Hyperbridge substrate chain using RPC calls to:
- `state_getStorage` for total supply from `balances.totalIssuance`
- `state_getStorage` for inactive issuance from `balances.inactiveIssuance`
- `state_getKeysPaged` and `state_queryStorageAt` for account locks from `balances.locks`

## Important Notes

- Token amounts are returned in human-readable token units (scaled down from raw blockchain units)
- There is only one BridgeTokenSupply entity with ID "hyperbridge-token-supply"
- All numeric fields support standard GraphQL filtering operations (greaterThan, lessThan, etc.)
- The API supports real-time queries and will return the most up-to-date supply information
