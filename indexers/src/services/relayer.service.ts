import { EthereumResult, EthereumTransaction } from "@subql/types-ethereum";
import { SupportedChain } from "../types/enums";
import { Relayer, Transfer } from "../types/models";
import {
  convertArrayToEnumListString,
  convertEnumListStringToArray,
} from "../utils/enum.helpers";
import { RelayerChainStatsService } from "./relayerChainStats.service";
import { getNativeCurrencyPrice } from "../utils/price.helpers";
import {
  PostRequestEventLog,
  PostResponseEventLog,
} from "../types/abi-interfaces/EthereumHostAbi";

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
        chains: convertArrayToEnumListString([
          chain,
        ]) as unknown as SupportedChain[],
        totalNumberOfMessagesSent: BigInt(0),
        totalFeesEarned: BigInt(0),
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
   *
   * @note All of the weird data type casting for relayer.networks is due to a limitation in SubQuery's datastore handling of arrays of enums
   *       See https://stackoverflow.com/questions/18234946/postgresql-insert-into-an-array-of-enums
   */
  static updateRelayerNetworksList(
    relayer: Relayer,
    chain: SupportedChain,
  ): Relayer {
    let chains_list = convertEnumListStringToArray(`${relayer.chains}`);

    if (!chains_list.includes(chain)) {
      chains_list.push(chain);
    }

    relayer.chains = convertArrayToEnumListString(
      chains_list,
    ) as unknown as SupportedChain[];

    return relayer;
  }

  /**
   * Computes relayer specific stats from the PostRequestHandled/PostResponseHandle event's transaction
   */
  static async handlePostRequestOrPostResponseHandledEvent(
    relayer_id: string,
    transaction: EthereumTransaction<EthereumResult>,
    chain: SupportedChain,
  ): Promise<void> {
    const { gasUsed, effectiveGasPrice } = await transaction.receipt();
    const ethPriceInUsd = await getNativeCurrencyPrice();

    const gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed);
    const usdFee = gasFee * BigInt(ethPriceInUsd);

    const relayer = await RelayerService.findOrCreate(relayer_id, chain);
    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
      relayer_id,
      chain,
    );

    relayer.totalNumberOfMessagesSent += BigInt(1);
    relayer_chain_stats.numberOfMessagesSent += BigInt(1);

    relayer_chain_stats.numberOfSuccessfulMessagesSent += BigInt(1);
    relayer_chain_stats.gasUsedForSuccessfulMessages += BigInt(gasUsed);
    relayer_chain_stats.gasFeeForSuccessfulMessages += BigInt(gasFee);
    relayer_chain_stats.usdGasFeeForSuccessfulMessages += BigInt(usdFee);

    Promise.all([await relayer_chain_stats.save(), await relayer.save()]);
  }

  /**
   * Computes relayer specific stats from the PostRequest/PostResponse event's transaction when the transaction fails
   */
  static async handlePostRequestOrResponseEvent(
    chain: SupportedChain,
    event: PostRequestEventLog | PostResponseEventLog,
  ): Promise<void> {
    const { transaction } = event;
    const { from: relayer_id } = transaction;
    const { status } = await transaction.receipt();

    if (status === false) {
      const { gasUsed, effectiveGasPrice } = await transaction.receipt();
      const ethPriceInUsd = await getNativeCurrencyPrice();

      const gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed);
      const usdFee = gasFee * BigInt(ethPriceInUsd);

      const relayer = await RelayerService.findOrCreate(relayer_id, chain);
      let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
        relayer_id,
        chain,
      );

      relayer.totalNumberOfMessagesSent += BigInt(1);
      relayer_chain_stats.numberOfMessagesSent += BigInt(1);

      relayer_chain_stats.numberOfFailedMessagesSent += BigInt(1);
      relayer_chain_stats.gasUsedForFailedMessages += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForFailedMessages += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForFailedMessages += BigInt(usdFee);

      Promise.all([await relayer_chain_stats.save(), await relayer.save()]);
    }
  }
}
