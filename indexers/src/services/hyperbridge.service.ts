import { HYPERBRIDGE_STATS_ENTITY_ID } from "../constants";
import { SupportedChain } from "../types";
import {
  PostRequestEventLog,
  PostResponseEventLog,
} from "../types/abi-interfaces/EthereumHostAbi";
import { HyperBridgeStats, Relayer, Transfer } from "../types/models";
import { HyperBridgeChainStatsService } from "./hyperbridgeChainStats.service";
import assert from "assert";
import { isHexString } from "ethers/lib/utils";
import { EthereumHostAbi__factory } from "../types/contracts";
import {
  HandlePostRequestsTransaction,
  HandlePostResponsesTransaction,
} from "../types/abi-interfaces/HandlerV1Abi";

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
        numberOfDeliveredMessages: BigInt(0),
        numberOfFailedDeliveries: BigInt(0),
        numberOfTimedOutMessages: BigInt(0),
        feesPayedOutToRelayers: BigInt(0),
        protocolFeesEarned: BigInt(0),
        totalTransfersIn: BigInt(0),
      });

      await stats.save();
    }

    return stats;
  }

  /**
   * Perform the necessary actions related to Hyperbridge stats when a PostRequest/PostResponse event is indexed
   */
  static async handlePostRequestOrResponseEvent(
    chain: SupportedChain,
    event: PostRequestEventLog | PostResponseEventLog,
  ): Promise<void> {
    assert(
      event.args,
      "No handlePostRequestEvent/handlePostResponseEvent args",
    );

    const { args, address } = event;
    let { data } = args;

    const protocolFee = await this.computeProtocolFeeFromHexData(address, data);

    Promise.all([
      await this.incrementProtocolFeesEarned(protocolFee, chain),
      await this.incrementNumberOfSentMessages(chain),
    ]);
  }

  /**
   * Perform the necessary actions related to Hyperbridge stats when a PostRequestHandled/PostResponseHandled event is indexed
   */
  static async handlePostRequestOrResponseHandledEvent(
    _relayer_id: string,
    chain: SupportedChain,
  ): Promise<void> {
    await this.incrementNumberOfDeliveredMessages(chain);
  }

  /**
   * Handle PostRequest or PostResponse transactions
   */
  static async handlePostRequestOrResponseTransaction(
    chain: SupportedChain,
    transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction,
  ): Promise<void> {
    const { status } = await transaction.receipt();

    if (status === false) {
      await this.incrementNumberOfFailedDeliveries(chain);
    }
  }

  /**
   * Increment the total number of messages sent on hyperbridge
   */
  static async incrementNumberOfSentMessages(
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
  static async incrementNumberOfDeliveredMessages(
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.numberOfDeliveredMessages += BigInt(1);

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.numberOfDeliveredMessages += BigInt(1);

    Promise.all([await chainStats.save(), await stats.save()]);
  }

  /**
   * Increment the number of failed deliveries by hyperbridge
   */
  static async incrementNumberOfFailedDeliveries(
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.numberOfFailedDeliveries += BigInt(1);

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.numberOfFailedDeliveries += BigInt(1);

    Promise.all([await chainStats.save(), await stats.save()]);
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
   * Increment the protocol fees earned by hyperbridge
   */
  static async incrementProtocolFeesEarned(
    amount: bigint,
    chain: SupportedChain,
  ): Promise<void> {
    let stats = await this.getStats();
    stats.protocolFeesEarned += amount;

    // Update the specific chain stats
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.protocolFeesEarned += amount;

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

    // Update the specific chain metrics
    let chainStats =
      await HyperBridgeChainStatsService.findOrCreateChainStats(chain);
    chainStats.totalTransfersIn += BigInt(transfer.amount);

    Promise.all([await chainStats.save(), await stats.save()]);
  }

  static async computeProtocolFeeFromHexData(
    contract_address: string,
    data: string,
  ): Promise<bigint> {
    data = isHexString(data) ? data.slice(2) : data;
    const noOfBytesInData = data.length / 2;
    const evmHostContract = EthereumHostAbi__factory.connect(
      contract_address,
      api,
    );
    const perByteFee = await evmHostContract.perByteFee();
    return perByteFee.mul(noOfBytesInData).toBigInt();
  }
}
