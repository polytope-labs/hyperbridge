import { HYPERBRIDGE_METRICS_ENTITY_ID } from "../constants";
import { SupportedChain, HyperBridgeChainMetrics } from "../types";

export class HyperBridgeChainMetricsService {
  /**
   * Find the HyperBridgeChainMetrics record for a chain, create it if it doesn't exist
   */
  static async findOrCreateChainMetrics(
    chain: SupportedChain,
  ): Promise<HyperBridgeChainMetrics> {
    let chainMetrics = await HyperBridgeChainMetrics.get(chain);

    if (typeof chainMetrics === "undefined") {
      chainMetrics = HyperBridgeChainMetrics.create({
        id: chain,
        hyperBridgeMetricsId: HYPERBRIDGE_METRICS_ENTITY_ID,
        totalTransfersIn: BigInt(0),
        feesEarned: BigInt(0),
        feesPayedOut: BigInt(0),
        postRequestsHandled: BigInt(0),
      });
      await chainMetrics.save();
    }

    return chainMetrics;
  }
}
