import { HYPERBRIDGE_METRICS_ENTITY_ID } from "../constants";
import { HyperBridgeMetrics } from "../types/models";

export class HyperBridgeService {
  /**
   * Get the HyperBridgeMetrics entity
   */
  static async getMetrics(): Promise<HyperBridgeMetrics> {
    let metrics = await HyperBridgeMetrics.get(HYPERBRIDGE_METRICS_ENTITY_ID);

    if (!metrics) {
      metrics = new HyperBridgeMetrics(
        HYPERBRIDGE_METRICS_ENTITY_ID,
        BigInt(0),
        BigInt(0),
      );
    }

    return metrics;
  }

  /**
   * Increment the number of post requests handled by hyperbridge
   */
  static async incrementNumberOfPostRequestsHandled(): Promise<void> {
    let metrics = await this.getMetrics();
    metrics.postRequestsHandled += BigInt(1);
    await metrics.save();
  }
}
