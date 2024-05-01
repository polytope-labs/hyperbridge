import { HYPERBRIDGE_STATS_ENTITY_ID } from "../constants";
import { SupportedChain } from "../types";
import { HyperBridgeStats, Relayer, Transfer } from "../types/models";
import { HyperBridgeChainStatsService } from "./hyperbridgeChainStats.service";
import { RelayerService } from "./relayer.service";

export class HyperBridgeService {
  /**
   * Get the HyperBridgeStats entity
   */
  static async getStats(): Promise<HyperBridgeStats> {
    let stats = await HyperBridgeStats.get(HYPERBRIDGE_STATS_ENTITY_ID);

    if (!stats) {
      stats = HyperBridgeStats.create({
        id: HYPERBRIDGE_STATS_ENTITY_ID,
        numberOfMessagesSent: BigInt(0),
        numberOfSuccessfulMessagesSent: BigInt(0),
        numberOfTimedOutMessages: BigInt(0),
        numberOfUniqueRelayers: BigInt(0),
        feesPayedOutToRelayers: BigInt(0),
        protocolFeesEarned: BigInt(0),
        totalTransfersIn: BigInt(0),
      });

      await stats.save();
    }

    return stats;
  }

  /**
   * Perform the necessary actions related to Hyperbridge stats when a PostRequestHandled/PostResponseHandled event is indexed
   */
  static async handlePostRequestOrResponseHandledEvent(
    relayer_id: string,
    chain: SupportedChain,
    transaction_status: boolean,
  ): Promise<void> {
    await this.incrementTotalNumberOfMessagesSent(chain);
    await this.updateNumberOfUniqueRelayers(relayer_id);

    if (transaction_status) {
      await this.incrementNumberOfSuccessfulMessagesSent(chain);
    }

    // Update the protocol fees earned
    // Get the message size in bytes
  }

  /**
   * Increment the total number of messages sent by hyperbridge
   */
  static async incrementTotalNumberOfMessagesSent(
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.numberOfMessagesSent += BigInt(1);

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.numberOfMessagesSent += BigInt(1);

    Promise.all([await chainStats.save(), await stats.save()]);
  }

  /**
   * Increment the number of successful messages handled by hyperbridge
   */
  static async incrementNumberOfSuccessfulMessagesSent(
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.numberOfSuccessfulMessagesSent += BigInt(1);

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.numberOfSuccessfulMessagesSent += BigInt(1);

    Promise.all([await chainStats.save(), await stats.save()]);
  }

  /**
   * Increment the number of unique relayers on Hyperbridge (if the relayer doesn't exist)
   */
  static async updateNumberOfUniqueRelayers(relayer_id: string): Promise<void> {
    let relayer = await Relayer.get(relayer_id);

    if (typeof relayer === "undefined") {
      let stats = await this.getStats();
      stats.numberOfUniqueRelayers += BigInt(1);
      await stats.save();
    }
  }

  /**
   * Increment the number of timed-out messages handled by hyperbridge
   */
  static async incrementNumberOfTimedOutMessagesSent(
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.numberOfTimedOutMessages += BigInt(1);

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.numberOfTimedOutMessages += BigInt(1);

    Promise.all([await chainStats.save(), await stats.save()]);
  }

  /**
   * Handle transfers out of the host account, incrementing the fees payed out to relayers
   */
  static async handleTransferOutOfHostAccounts(
    transfer: Transfer,
    chain: SupportedChain,
  ): Promise<void> {
    let relayer = await Relayer.get(transfer.to);

    if (typeof relayer !== "undefined") {
      let stats = await this.getStats();
      let chainStats =
        await HyperBridgeChainStatsService.findOrCreateChainStats(chain);

      stats.feesPayedOutToRelayers += BigInt(transfer.amount);
      chainStats.feesPayedOutToRelayers += BigInt(transfer.amount);

      Promise.all([await chainStats.save(), await stats.save()]);
    }
  }

  /**
   * Increment the total amount transferred to hyperbridge (protocol fees + relayer fees)
   */
  static async updateTotalTransfersIn(
    transfer: Transfer,
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.totalTransfersIn += BigInt(transfer.amount);
    stats.protocolFeesEarned =
      stats.totalTransfersIn - stats.feesPayedOutToRelayers;

    // Update the specific chain metrics
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.totalTransfersIn += BigInt(transfer.amount);
    chainStats.protocolFeesEarned =
      chainStats.totalTransfersIn - chainStats.feesPayedOutToRelayers;

    Promise.all([await chainStats.save(), await stats.save()]);
  }
}
