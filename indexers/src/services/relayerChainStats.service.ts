import { RelayerStatsPerChain, SupportedChain } from "../types";

export class RelayerChainStatsService {
  /*
   * Find the RelayerChainMetrics record for a relayer on a chain, create it if it doesn't exist
   */
  static async findOrCreate(
    relayer_id: string,
    chain: SupportedChain,
  ): Promise<RelayerStatsPerChain> {
    let id = `${relayer_id}-${chain}`;
    let metrics = await RelayerStatsPerChain.get(id);

    if (!metrics) {
      metrics = RelayerStatsPerChain.create({
        id,
        postRequestsHandled: BigInt(0),
        relayerId: relayer_id,
        chain,
        failedPostRequestsHandled: BigInt(0),
        successfulPostRequestsHandled: BigInt(0),
        gasUsedForFailedPostRequests: BigInt(0),
        gasUsedForSuccessfulPostRequests: BigInt(0),
        gasFeeForFailedPostRequests: BigInt(0),
        gasFeeForSuccessfulPostRequests: BigInt(0),
        usdGasFeeForFailedPostRequests: BigInt(0),
        usdGasFeeForSuccessfulPostRequests: BigInt(0),
        feesEarned: BigInt(0),
      });
      await metrics.save();
    }

    return metrics;
  }
}
