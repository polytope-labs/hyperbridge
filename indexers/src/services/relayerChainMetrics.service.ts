import { RelayerChainMetrics, SupportedChain } from "../types";

export class RelayerChainMetricsService {
  /*
   * Find the RelayerChainMetrics record for a relayer on a chain, create it if it doesn't exist
   */
  static async findOrCreate(
    relayer_id: string,
    chain: SupportedChain,
  ): Promise<RelayerChainMetrics> {
    let id = `${relayer_id}-${chain}`;
    let metrics = await RelayerChainMetrics.get(id);

    if (!metrics) {
      metrics = RelayerChainMetrics.create({
        id,
        postRequestsHandled: BigInt(0),
        relayerId: relayer_id,
        chain,
        failedPostRequests: BigInt(0),
        successfulPostRequests: BigInt(0),
        gasUsedForFailedPostRequests: BigInt(0),
        gasUsedForSuccessfulPostRequests: BigInt(0),
        gasCostForFailedPostRequests: BigInt(0),
        gasCostForSuccessfulPostRequests: BigInt(0),
        feesEarned: BigInt(0),
      });
      await metrics.save();
    }

    return metrics;
  }
}
