import { handlePostRequestHandledCount } from '../src/graphQl/postRequestHandledCount'

describe('Post requests handled count', async () => {
  it('should get the accurate number of post requests handled', async () => {
    const subgraphData = await handlePostRequestHandledCount()

    console.log('Total post requests handled ::   ', subgraphData.data.postRequestHandledCounts[0].value)
  })
})
