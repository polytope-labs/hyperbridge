import { HYPERBRIDGE_METRICS_ENTITY_ID } from "../constants";
import { SupportedChain } from "../types";
import { HyperBridgeMetrics, Transfer } from "../types/models";
import { HyperBridgeChainMetricsService } from "./hyperbridgeChainMetrics.service";

export class HyperBridgeService {
  /**
   * Get the HyperBridgeMetrics entity
   */
  static async getMetrics(): Promise<HyperBridgeMetrics> {
    let metrics = await HyperBridgeMetrics.get(HYPERBRIDGE_METRICS_ENTITY_ID);

    if (!metrics) {
      metrics = HyperBridgeMetrics.create({
        id: HYPERBRIDGE_METRICS_ENTITY_ID,
        postRequestsHandled: BigInt(0),
        feesPayedOut: BigInt(0),
        feesEarned: BigInt(0),
        totalTransfersIn: BigInt(0),
        perChainMetrics: [],
      });

      await metrics.save();
    }

    return metrics;
  }

  /**
   * Increment the number of post requests handled by hyperbridge
   */
  static async incrementNumberOfPostRequestsHandled(
    chain: SupportedChain,
  ): Promise<void> {
    let metrics = await this.getMetrics();
    metrics.postRequestsHandled += BigInt(1);

    // Update the specific chain metrics
    let chainMetrics =
      await HyperBridgeChainMetricsService.findOrCreateChainMetrics(
        chain,
        metrics,
      );
    chainMetrics.postRequestsHandled += BigInt(1);

    HyperBridgeChainMetricsService.updateChainMetrics(metrics, chainMetrics);
    await metrics.save();
  }

  /**
   * Increment the total amount of fees payed out by hyperbridge to relayers
   */
  static async updateFeesPayedOut(
    transfer: Transfer,
    chain: SupportedChain,
  ): Promise<void> {
    let metrics = await this.getMetrics();
    metrics.feesPayedOut += BigInt(transfer.amount);
    metrics.feesEarned = metrics.totalTransfersIn - metrics.feesPayedOut;

    // Update the specific chain metrics
    let chainMetrics =
      await HyperBridgeChainMetricsService.findOrCreateChainMetrics(
        chain,
        metrics,
      );
    chainMetrics.feesPayedOut += BigInt(transfer.amount);
    chainMetrics.feesEarned =
      chainMetrics.totalTransfersIn - chainMetrics.feesPayedOut;

    HyperBridgeChainMetricsService.updateChainMetrics(metrics, chainMetrics);
    await metrics.save();
  }

  /**
   * Increment the total amount transferred to hyperbridge (protocol fees + relayer fees)
   */
  static async updateTotalTransfersIn(
    transfer: Transfer,
    chain: SupportedChain,
  ): Promise<void> {
    let metrics = await this.getMetrics();
    metrics.totalTransfersIn += BigInt(transfer.amount);
    metrics.feesEarned = metrics.totalTransfersIn - metrics.feesPayedOut;

    // Update the specific chain metrics
    let chainMetrics =
      await HyperBridgeChainMetricsService.findOrCreateChainMetrics(
        chain,
        metrics,
      );
    chainMetrics.totalTransfersIn += BigInt(transfer.amount);
    chainMetrics.feesEarned =
      chainMetrics.totalTransfersIn - chainMetrics.feesPayedOut;

    HyperBridgeChainMetricsService.updateChainMetrics(metrics, chainMetrics);
    await metrics.save();
  }
}
