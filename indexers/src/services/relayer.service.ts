import { EthereumResult, EthereumTransaction } from "@subql/types-ethereum";
import { SupportedChain } from "../types/enums";
import { Relayer, Transfer } from "../types/models";
import { RelayerChainStatsService } from "./relayerChainStats.service";
import { getNativeCurrencyPrice } from "../utils/price.helpers";
import {
  HandlePostRequestsTransaction,
  HandlePostResponsesTransaction,
} from "../types/abi-interfaces/HandlerV1Abi";
import { ETHEREUM_L2_SUPPORTED_CHAINS } from "../constants";

export class RelayerService {
  /**
   * Find a relayer by its id or create a new one if it doesn't exist
   */
  static async findOrCreate(
    relayer_id: string,
    chain: SupportedChain,
  ): Promise<Relayer> {
    let relayer = await Relayer.get(relayer_id);

    if (typeof relayer === "undefined") {
      relayer = Relayer.create({
        id: relayer_id,
      });

      await relayer.save();
    }

    return relayer;
  }

  /**
   * Update the total fees earned by a relayer
   * Fees earned by a relayer == Sum of all transfers to the relayer from the hyperbridge host address
   */
  static async updateFeesEarned(transfer: Transfer): Promise<void> {
    let relayer = await Relayer.get(transfer.to);
    if (relayer) {
      let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
        relayer.id,
        transfer.chain,
      );

      relayer_chain_stats.feesEarned += transfer.amount;

      Promise.all([relayer.save(), relayer_chain_stats.save()]);
    }
  }

  /**
   * Computes relayer specific stats from the PostRequestHandled/PostResponseHandle event's transaction
   */
  static async handlePostRequestOrPostResponseHandledEvent(
    relayer_id: string,
    _transaction: EthereumTransaction<EthereumResult>,
    chain: SupportedChain,
  ): Promise<void> {
    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
      relayer_id,
      chain,
    );

    relayer_chain_stats.numberOfMessagesDelivered += BigInt(1);
    relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1);

    Promise.all([await relayer_chain_stats.save()]);
  }

  /**
   * Computes relayer specific stats from the handlePostRequest/handlePostResponse transactions on the handlerV1 contract
   */
  static async handlePostRequestOrResponseTransaction(
    chain: SupportedChain,
    transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction,
  ): Promise<void> {
    const { from: relayer_id } = transaction;
    const receipt = await transaction.receipt();
    const { status, gasUsed, effectiveGasPrice } = receipt;

    const nativeCurrencyPrice = await getNativeCurrencyPrice(chain);

    let gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed);

    // Add the L1 Gas Used for L2 chains
    if (ETHEREUM_L2_SUPPORTED_CHAINS.includes(chain)) {
      if (!(receipt as any).l1Fee) {
        logger.error(
          `Could not find l1Fee in transaction receipt: ${JSON.stringify({ chain, transactionHash: transaction.hash })}`,
        );
      }
      const l1Fee = BigInt((receipt as any).l1Fee ?? 0);
      gasFee += l1Fee;
    }

    const _gasFeeInEth = Number(gasFee) / Number(BigInt(10 ** 18));
    const usdFee = (gasFee * nativeCurrencyPrice) / BigInt(10 ** 18);

    const relayer = await RelayerService.findOrCreate(relayer_id, chain);
    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
      relayer_id,
      chain,
    );

    if (status === true) {
      relayer_chain_stats.gasUsedForSuccessfulMessages += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForSuccessfulMessages += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForSuccessfulMessages += usdFee;
    } else {
      relayer_chain_stats.numberOfMessagesDelivered += BigInt(1);
      relayer_chain_stats.numberOfFailedMessagesDelivered += BigInt(1);

      relayer_chain_stats.gasUsedForFailedMessages += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForFailedMessages += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForFailedMessages += usdFee;
    }

    Promise.all([await relayer_chain_stats.save(), await relayer.save()]);
  }
}
