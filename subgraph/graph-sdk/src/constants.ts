// all constant would go into this folder
import { ApolloClient, InMemoryCache } from '@apollo/client'
export const SUBGRAPH_API_URL = 'https://api.studio.thegraph.com/query/66742/hyperbridgeupdated/v0.0.3'

export const client = new ApolloClient({
  uri: SUBGRAPH_API_URL,
  cache: new InMemoryCache(),
})
