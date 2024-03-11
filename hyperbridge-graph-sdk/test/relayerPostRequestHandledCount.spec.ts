import { handleRelayerPostRequestHandledCount } from '../src/graphQl/relayerPostRequestHandledCount'

describe('Relayer post requests handled count', async () => {
  it('should get the accurate number of post requests handled by a relayer', async () => {
    const relayerAddress = '0x7d72983fedc1f332e55006fea2a2afc148f66142'

    const subgraphData = await handleRelayerPostRequestHandledCount(relayerAddress)

    console.log(
      `Total relayer post requests handled by ${relayerAddress} ::`,
      subgraphData.data.relayerPostRequestHandledCount.value
    )
  })
})
