# Hyperbridge Subgraph
The

## Subgraph Endpoint

Synced at: https://thegraph.com/studio/subgraph/hyperbridgeupdated/playground

Available at: https://api.studio.thegraph.com/query/66742/hyperbridgeupdated/v0.0.3

Docs reference: https://docs.google.com/document/d/1ja_hWYOfu764GIwPT-lONJW1aqPFEMN6lmcRTZ_Bslk/edit?usp=sharing

Subgraph Schema: https://github.com/polytope-labs/hyperbridge/blob/d24057af4670673ca5bb1c408a838217e8a1c3e3/subgraph/schema.graphql

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

### 3. Amount Earned by a relayer
To get the amount earned by a given relayer, we calculated the total amount transferred from the EVMHost to the specified relayer. This is available on the `transferPairTotals` endpoint. In our query, we make use of the `where` clause, given the <b>host address</b> as the `from` parameter and the <b>relayer address</b> as the `to` parameter. See usage below:

Query:
```sh
{
  transferPairTotals(
    where: {to: "0x7d72983fedc1f332e55006fea2a2afc148f66142", from: "0xe4226c474A6f4BF285eA80c2f01c0942B04323e5"}
  ) {
    from
    id
    to
    totalAmount
  }
}
```

Response:
```sh
{
  "data": {
    "transferPairTotals": [
      {
        "from": "0xe4226c474a6f4bf285ea80c2f01c0942b04323e5",
        "id": "0xe4226c474a6f4bf285ea80c2f01c0942b04323e5-0x7d72983fedc1f332e55006fea2a2afc148f66142",
        "to": "0x7d72983fedc1f332e55006fea2a2afc148f66142",
        "totalAmount": "1800000000000000000000"
      }
    ]
  }
}
```

### 4. Total fees earned by Hyperbridge
Calculating the total fees earned by Hyperbridge requires some manual calculation where we:
- First get the sum of the total amount of feeToken transferred into the EVMHost.
- Secondly we get the sum of the total amount in fees emitted by the PostRequestEvents
- Lastly we deduct totalPostEventFees from the total amount transferred into the EVMHost. The balance is the amount of fees earned Hyperbridge.

See Usage Below:

For `inTransferTotal`: this takes the host address as parameter for the id field and returns the total amount of feeToken transferred to the host.

Query:
```sh
{
  inTransferTotal(id: "0xe4226c474a6f4bf285ea80c2f01c0942b04323e5") {
    id
    totalAmountTransferredIn
  }
}
```

Response:
```sh
{
  "data": {
    "inTransferTotal": {
      "id": "0xe4226c474a6f4bf285ea80c2f01c0942b04323e5",
      "totalAmountTransferredIn": "69829815000000001000000"
    }
  }
}
```

For `requestEventFeeTotals`: this returns the total fees (relayer fees) emitted by the PostRequestEvent

Query:
```sh
{
  requestEventFeeTotals {
    id
    totalRequestFee
  }
}
```

Response:
```sh
{
  "data": {
    "requestEventFeeTotals": [
      {
        "id": "1",
        "totalRequestFee": "69480000000000001000000"
      }
    ]
  }
}
```

Fee earned by Hyperbridge will be `totalAmountTransferredIn - totalRequestFee`. This calculation should be done on the frontend.

The result of the subtraction will be:
```
69829815000000001000000 - 69480000000000001000000 = 34981500000000000000
```

#### Total fees earned by Hyperbridge = 34981500000000000000

## Frontend Usage
Usage of the graphql endpoints with any graphql client of choice

### Hyperbridge Fees
```javascript
export async function handleHyperbridgeFeesEarned(hostAddress: string): Promise<number> {
  const modifiedHostAddress = hostAddress.toLowerCase()

  const totalAmountTransferredIntoHost = await handleInTransferTotal(modifiedHostAddress)
  const totalRelayerFeeEmittedByHost = await handleRequestEventFeeTotal()

  const hyperbridgeFeesEarned = totalAmountTransferredIntoHost - totalRelayerFeeEmittedByHost

  return hyperbridgeFeesEarned
}

async function handleInTransferTotal(hostAddress: string): Promise<number> {
  const operationName = QueryOptions.InTransferTotal

  const response = await client.query({
    query: gql`
      query ${operationName}($hostId: ID!) {
        inTransferTotal(id: $hostId) {
          id
          totalAmountTransferredIn
        }
      }
    `,
    variables: {
      hostId: hostAddress,
    },
  })

  return response.data.inTransferTotal.totalAmountTransferredIn
}

async function handleRequestEventFeeTotal(): Promise<number> {
  const operationName = QueryOptions.RequestEventFeeTotal

  const response = await client.query({
    query: gql`
        query ${operationName} {
          requestEventFeeTotals {
            id
            totalRequestFee
          }
        }
      `,
  })

  return response.data.requestEventFeeTotals[0].totalRequestFee
}
```

### Post Requests Handled Count
```javascript
export async function handlePostRequestHandledCount() {
  const operationName = QueryOptions.PostRequestHandledCount

  const response = await client.query({
    query: gql`
      query ${operationName} {
        postRequestHandledCounts {
          id
          value
        }
      }
    `,
  })

  return response
}
```

### Relayer Post Requests Handled Count
```javascript
export async function handleRelayerPostRequestHandledCount(relayerAddress: string) {
  const operationName = QueryOptions.RelayerPostRequestHandledCount
  const modifiedRelayerAddress = relayerAddress.toLowerCase()

  const response = await client.query({
    query: gql`
      query ${operationName}($relayerId: ID!) {
        relayerPostRequestHandledCount(id: $relayerId) {
          id
          value
        }
      }
    `,
    variables: {
      relayerId: modifiedRelayerAddress,
    },
  })

  return response
}
```

### Relayer Amount Earned
```javascript
export async function handleRelayerAmountEarned(hostAddress: string, relayerAddress: string) {
  const operationName = QueryOptions.RelayerPostRequestHandledCount

  const modifiedHostAddress = hostAddress.toLowerCase()
  const modifiedRelayerAddress = relayerAddress.toLowerCase()

  const response = await client.query({
    query: gql`
        query ${operationName}($hostAddress: String!, $relayerAddress: String!) {
          transferPairTotals(
            where: {from: $hostAddress, to: $relayerAddress}
          ) {
            from
            id
            to
            totalAmount
          }
        }
      `,
    variables: {
      hostAddress: modifiedHostAddress,
      relayerAddress: modifiedRelayerAddress,
    },
  })

  return response
}
```

### Query Option Enum
```javascript
export enum QueryOptions {
  TransferTotal = 'TransferTotal',
  TransferPairTotal = 'TransferPairTotal',
  InTransferTotal = 'InTransferTotal',
  RequestEventFeeTotal = 'RequestEventFeeTotal',
  PostRequestHandledCount = 'PostRequestHandledCount',
  RelayerPostRequestHandledCount = 'RelayerPostRequestHandledCount',
}

```