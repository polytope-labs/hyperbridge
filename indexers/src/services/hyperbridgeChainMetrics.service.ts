import {
  SupportedChain,
  HyperBridgeMetrics,
  HyperBridgeChainMetrics,
} from "../types";

export class HyperBridgeChainMetricsService {
  /**
   * Find the HyperBridgeChainMetrics record for a chain, create it if it doesn't exist
   */
  static async findOrCreateChainMetrics(
    chain: SupportedChain,
    metrics: HyperBridgeMetrics,
  ): Promise<HyperBridgeChainMetrics> {
    let chainMetrics = metrics.perChainMetrics.find(
      (chainMetrics) => chainMetrics.chain == (chain as string),
    );

    if (typeof chainMetrics === "undefined") {
      chainMetrics = {
        chain: chain as string,
        totalTransfersIn: BigInt(0),
        feesEarned: BigInt(0),
        feesPayedOut: BigInt(0),
        postRequestsHandled: BigInt(0),
      };
      metrics.perChainMetrics.push(chainMetrics);
    }

    await metrics.save();
    return chainMetrics;
  }

  /**
   * Update a chain's metrics in the hyperbridge metrics object
   * This does not save the metrics object to the database, the caller is responsible for that
   */
  static updateChainMetrics(
    metrics: HyperBridgeMetrics,
    updatedChainMetrics: HyperBridgeChainMetrics,
  ): HyperBridgeMetrics {
    metrics.perChainMetrics = metrics.perChainMetrics.map((chainMetrics) => {
      if (updatedChainMetrics.chain == chainMetrics.chain) {
        return updatedChainMetrics;
      } else {
        return chainMetrics;
      }
    });

    return metrics;
  }
}
