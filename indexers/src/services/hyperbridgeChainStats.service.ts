import { HyperBridgeChainStats } from "../types";

export class HyperBridgeChainStatsService {
  /**
   * Find the HyperBridgeChainStats record for a chain, create it if it doesn't exist
   */
  static async findOrCreateChainStats(
    chain: string
  ): Promise<HyperBridgeChainStats> {
    let chainStats = await HyperBridgeChainStats.get(chain);

    if (typeof chainStats === "undefined") {
      chainStats = HyperBridgeChainStats.create({
        id: chain,
        totalTransfersIn: BigInt(0),
        protocolFeesEarned: BigInt(0),
        feesPayedOutToRelayers: BigInt(0),
        numberOfMessagesSent: BigInt(0),
        numberOfDeliveredMessages: BigInt(0),
        numberOfFailedDeliveries: BigInt(0),
        numberOfTimedOutMessages: BigInt(0),
      });
      await chainStats.save();
    }

    return chainStats;
  }
}
