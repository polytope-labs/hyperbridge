# Hyperbridge Subgraph
The

## Subgraph Endpoint

Synced at: https://thegraph.com/studio/subgraph/hyperbridgeupdated/playground

Available at: https://api.studio.thegraph.com/query/66742/hyperbridgeupdated/v0.0.3

## Queries and Response

### 1. Number of post request handled

The `postRequestHandledCounts` returns the total number of post requests handled totally.

Query:
```sh
{
  postRequestHandledCounts {
    id
    value
  }
}
```

Response:
```sh
{
  "data": {
    "postRequestHandledCounts": [
      {
        "id": "1",
        "value": "7489"
      }
    ]
  }
}
```

### 2. Number of post request handled by a relayer/relayers
