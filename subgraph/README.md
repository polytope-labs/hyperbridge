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

There are two endpoints for this query, the `relayerPostRequestHandledCounts` returns an array of object of all the relayers and the number of requests they handled, while `relayerPostRequestHandledCount` returns the request handled count of just a relayer given the relayer address as an `id` parameter. See example below:

For `relayerPostRequestHandledCounts`:

Query:
```sh
{
  relayerPostRequestHandledCounts {
    id
    value
  }
}
```

Response:
```sh
{
  "data": {
    "relayerPostRequestHandledCounts": [
      {
        "id": "0x1f134e8104de3a18cf617649bf578ca32bde33b9",
        "value": "200"
      },
      {
        "id": "0x29cbf28f4971dfbc67eeea0203a9ab15be9780ff",
        "value": "410"
      },
      {
        "id": "0x484a0aa729e859490aae6ab45eab7ddc23df6e8f",
        "value": "671"
      },
      {
        "id": "0x651c34ffbf63d30841b4f433d43342d9dc8e28a9",
        "value": "301"
      },
      {
        "id": "0x7249de13688e90ae53b88aac69f0c75848b3396d",
        "value": "502"
      },
      {
        "id": "0x72994b4e09e9b59e8e1f78365e91792bdbba8072",
        "value": "100"
      },
      {
        "id": "0x7c76b7a9f23368aeae13bac933483a878a847f0c",
        "value": "200"
      },
      {
        "id": "0x7d72983fedc1f332e55006fea2a2afc148f66142",
        "value": "1202"
      },
      {
        "id": "0x89b60e639bbe657f28678b4ce362ac0165a98990",
        "value": "802"
      },
      {
        "id": "0x9232e148f4afcddae7cc67d5f1c2246dbe8049f5",
        "value": "100"
      },
      {
        "id": "0x97c3e3acf0211b0b41a7a06f09bf76670d8853e6",
        "value": "2200"
      },
      {
        "id": "0xbd4b60d7305bdc8bad6516a8b3432b459d5c797d",
        "value": "401"
      },
      {
        "id": "0xde013f6e7b9031f091835e125571f3d251baacfe",
        "value": "400"
      }
    ]
  }
```

For `relayerPostRequestHandledCount`:

Query:
```sh
{
  relayerPostRequestHandledCount(id: "0xde013f6e7b9031f091835e125571f3d251baacfe") {
    id
    value
  }
}
```

Response:
```sh
{
  "data": {
    "relayerPostRequestHandledCount": {
      "id": "0xde013f6e7b9031f091835e125571f3d251baacfe",
      "value": "400"
    }
  }
}
```