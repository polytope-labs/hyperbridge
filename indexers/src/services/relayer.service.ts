import { EthereumResult, EthereumTransaction } from "@subql/types-ethereum";
import { SupportedChain } from "../types/enums";
import { Relayer, Transfer } from "../types/models";
import {
  convertArrayToEnumListString,
  convertEnumListStringToArray,
} from "../utils/enum.helpers";
import { RelayerChainStatsService } from "./relayerChainStats.service";
import { getCurrentEthPriceInUsd } from "../utils/price.helpers";

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
        postRequestsHandled: BigInt(0),
        totalFeesEarned: BigInt(0),
      });

      await relayer.save();
    }

    return relayer;
  }
  /**
   * Increment the number of post requests handled by a relayer
   */
  static async incrementNumberOfPostRequestsHandled(
    relayer_id: string,
    chain: SupportedChain,
  ): Promise<void> {
    let relayer = await RelayerService.findOrCreate(relayer_id, chain);

    relayer.postRequestsHandled += BigInt(1);
    relayer = RelayerService.updateRelayerNetworksList(relayer, chain);

    await relayer.save();
  }

  /**
   * Update the total fees earned by a relayer
   * Fees earned by a relayer == Sum of all transfers to the relayer from the hyperbridge host address
   */
  static async updateFeesEarned(transfer: Transfer): Promise<void> {
    let relayer = await Relayer.get(transfer.to);
    if (relayer) {
      relayer.totalFeesEarned += transfer.amount;
      relayer = RelayerService.updateRelayerNetworksList(
        relayer,
        transfer.chain,
      );

      await relayer.save();
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
   * Computes relayer specific stats from the PostRequestHandled event's transaction
   */
  static async handlePostRequestHandledTransaction(
    relayer_id: string,
    transaction: EthereumTransaction<EthereumResult>,
    chain: SupportedChain,
  ): Promise<void> {
    const { status, gasUsed, effectiveGasPrice } = await transaction.receipt();
    const ethPriceInUsd = await getCurrentEthPriceInUsd();

    const gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed);
    const usdFee = gasFee * BigInt(ethPriceInUsd);

    const relayer = await RelayerService.findOrCreate(relayer_id, chain);
    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
      relayer_id,
      chain,
    );

    relayer_chain_stats.postRequestsHandled += BigInt(1);

    if (status) {
      // Handle successful requests
      relayer_chain_stats.successfulPostRequestsHandled += BigInt(1);
      relayer_chain_stats.gasUsedForSuccessfulPostRequests += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForSuccessfulPostRequests += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForSuccessfulPostRequests += BigInt(usdFee);
    } else {
      // Handle failed requests
      relayer_chain_stats.failedPostRequestsHandled += BigInt(1);
      relayer_chain_stats.gasUsedForFailedPostRequests += BigInt(gasUsed);
      relayer_chain_stats.gasFeeForFailedPostRequests += BigInt(gasFee);
      relayer_chain_stats.usdGasFeeForSuccessfulPostRequests += BigInt(usdFee);
    }

    await relayer_chain_stats.save();
  }
}
