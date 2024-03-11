import { gql } from '@apollo/client'
import { client } from '../constants'
import { QueryOptions } from '../types'

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
