import { EthereumResult, EthereumTransaction } from "@subql/types-ethereum";
import { SupportedChain } from "../types/enums";
import { Relayer, Transfer } from "../types/models";
import { RelayerChainStatsService } from "./relayerChainStats.service";
import { getNativeCurrencyPrice } from "../utils/price.helpers";
import {
  HandlePostRequestsTransaction,
  HandlePostResponsesTransaction,
} from "../types/abi-interfaces/HandlerV1Abi";
import { HyperBridgeService } from "./hyperbridge.service";
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
        chains: [chain],
        totalNumberOfMessagesDelivered: BigInt(0),
        totalNumberOfFailedMessagesDelivered: BigInt(0),
        totalNumberOfSuccessfulMessagesDelivered: BigInt(0),
        totalFeesEarned: BigInt(0),
      });

      await relayer.save();
      await HyperBridgeService.incrementNumberOfUniqueRelayers(chain, true);
    } else {
      if (!relayer.chains.includes(chain)) {
        relayer = this.updateRelayerNetworksList(relayer, chain);
        await HyperBridgeService.incrementNumberOfUniqueRelayers(chain, false);

        await relayer.save();
      }
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
      relayer = RelayerService.updateRelayerNetworksList(
        relayer,
        transfer.chain,
      );

      let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
        relayer.id,
        transfer.chain,
      );

      relayer.totalFeesEarned += transfer.amount;
      relayer_chain_stats.feesEarned += transfer.amount;

      Promise.all([relayer.save(), relayer_chain_stats.save()]);
    }
  }

  /**
   * Update the list of networks supported by a relayer.
   * This fn does not save the relayer to the store. It is the responsibility of the caller to save the relayer after calling this fn.
   */
  static updateRelayerNetworksList(
    relayer: Relayer,
    chain: SupportedChain,
  ): Relayer {
    let chains_list = relayer.chains;

    if (!chains_list.includes(chain)) {
      chains_list.push(chain);
    }

    relayer.chains = chains_list;

    return relayer;
  }

  /**
   * Computes relayer specific stats from the PostRequestHandled/PostResponseHandle event's transaction
   */
  static async handlePostRequestOrPostResponseHandledEvent(
    relayer_id: string,
    _transaction: EthereumTransaction<EthereumResult>,
    chain: SupportedChain,
  ): Promise<void> {
    const relayer = await RelayerService.findOrCreate(relayer_id, chain);
    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
      relayer_id,
      chain,
    );

    relayer.totalNumberOfMessagesDelivered += BigInt(1);
    relayer.totalNumberOfSuccessfulMessagesDelivered += BigInt(1);

    relayer_chain_stats.numberOfMessagesDelivered += BigInt(1);
    relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1);

    Promise.all([await relayer_chain_stats.save(), await relayer.save()]);
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
      relayer.totalNumberOfMessagesDelivered += BigInt(1);
      relayer.totalNumberOfFailedMessagesDelivered += BigInt(1);
      relayer_chain_stats.numberOfMessagesDelivered += BigInt(1);
      relayer_chain_stats.numberOfFailedMessagesDelivered += BigInt(1);

      relayer_chain_stats.gasUsedForFailedMessages += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForFailedMessages += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForFailedMessages += usdFee;
    }

    Promise.all([await relayer_chain_stats.save(), await relayer.save()]);
  }
}
