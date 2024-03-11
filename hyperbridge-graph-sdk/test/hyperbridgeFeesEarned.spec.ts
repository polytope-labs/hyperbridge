import { handleHyperbridgeFeesEarned } from '../src/graphQl/hyperbridgeFeesEarned'

describe('Hyperbridge fees earned', async () => {
  it('should get the accurate amount of fees earned by hyperbridge', async () => {
    const hostAddress = '0xe4226c474A6f4BF285eA80c2f01c0942B04323e5'

    const hyperbridgeFeesEarned = await handleHyperbridgeFeesEarned(hostAddress)

    console.log(`Amount earned by Hyperbridge ::`, hyperbridgeFeesEarned)
  })
})
