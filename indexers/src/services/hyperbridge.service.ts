import {
  PostRequestEventLog,
  PostResponseEventLog,
} from "../types/abi-interfaces/EthereumHostAbi";
import { Relayer, Transfer } from "../types/models";
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
   * Perform the necessary actions related to Hyperbridge stats when a PostRequest/PostResponse event is indexed
   */
  static async handlePostRequestOrResponseEvent(
    chain: string,
    event: PostRequestEventLog | PostResponseEventLog
  ): Promise<void> {
    assert(
      event.args,
      "No handlePostRequestEvent/handlePostResponseEvent args"
    );

    const { args, address } = event;
    let { body } = args;

    const protocolFee = await this.computeProtocolFeeFromHexData(address, body);

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
    chain: string
  ): Promise<void> {
    await this.incrementNumberOfDeliveredMessages(chain);
  }

  /**
   * Handle PostRequest or PostResponse transactions
   */
  static async handlePostRequestOrResponseTransaction(
    chain: string,
    transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction
  ): Promise<void> {
    const { status } = await transaction.receipt();

    if (status === false) {
      await this.incrementNumberOfFailedDeliveries(chain);
    }
  }

  /**
   * Increment the total number of messages sent on hyperbridge
   */
  static async incrementNumberOfSentMessages(chain: string): Promise<void> {
    // Update the specific chain stats
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.numberOfMessagesSent += BigInt(1);

    Promise.all([await chainStats.save()]);
  }

  /**
   * Increment the number of successful messages handled by hyperbridge
   */
  static async incrementNumberOfDeliveredMessages(
    chain: string
  ): Promise<void> {
    // Update the specific chain stats
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.numberOfDeliveredMessages += BigInt(1);

    await chainStats.save();
  }

  /**
   * Increment the number of failed deliveries by hyperbridge
   */
  static async incrementNumberOfFailedDeliveries(chain: string): Promise<void> {
    // Update the specific chain stats
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.numberOfFailedDeliveries += BigInt(1);

    await chainStats.save();
  }

  /**
   * Increment the number of timed-out messages handled by hyperbridge
   */
  static async incrementNumberOfTimedOutMessagesSent(
    chain: string
  ): Promise<void> {
    // Update the specific chain stats
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.numberOfTimedOutMessages += BigInt(1);

    await chainStats.save();
  }

  /**
   * Increment the protocol fees earned by hyperbridge
   */
  static async incrementProtocolFeesEarned(
    amount: bigint,
    chain: string
  ): Promise<void> {
    // Update the specific chain stats
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.protocolFeesEarned += amount;

    await chainStats.save();
  }

  /**
   * Handle transfers out of the host account, incrementing the fees payed out to relayers
   */
  static async handleTransferOutOfHostAccounts(
    transfer: Transfer,
    chain: string
  ): Promise<void> {
    let relayer = await Relayer.get(transfer.to);

    if (typeof relayer !== "undefined") {
      let chainStats =
        await HyperBridgeChainStatsService.findOrCreateChainStats(chain);

      chainStats.feesPayedOutToRelayers += BigInt(transfer.amount);

      await chainStats.save();
    }
  }

  /**
   * Increment the total amount transferred to hyperbridge (protocol fees + relayer fees)
   */
  static async updateTotalTransfersIn(
    transfer: Transfer,
    chain: string
  ): Promise<void> {
    // Update the specific chain metrics
    let chainStats = await HyperBridgeChainStatsService.findOrCreateChainStats(
      chain
    );
    chainStats.totalTransfersIn += BigInt(transfer.amount);

    await chainStats.save();
  }

  static async computeProtocolFeeFromHexData(
    contract_address: string,
    data: string
  ): Promise<bigint> {
    data = isHexString(data) ? data.slice(2) : data;
    const noOfBytesInData = data.length / 2;
    const evmHostContract = EthereumHostAbi__factory.connect(
      contract_address,
      api
    );
    const perByteFee = await evmHostContract.perByteFee();
    return perByteFee.mul(noOfBytesInData).toBigInt();
  }
}
