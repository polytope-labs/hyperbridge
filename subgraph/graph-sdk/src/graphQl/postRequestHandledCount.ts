import { gql } from '@apollo/client'
import { client } from '../constants'
import { QueryOptions } from '../types'

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
