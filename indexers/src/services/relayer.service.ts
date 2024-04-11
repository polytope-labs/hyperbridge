import { SupportedChain } from "../types/enums";
import { Relayer, Transfer } from "../types/models";

export class RelayerService {
  /**
   * Increment the number of post requests handled by a relayer
   */
  static async incrementNumberOfPostRequestsHandled(
    relayer_id: string,
    network: SupportedChain,
  ): Promise<void> {
    let relayer = await Relayer.get(relayer_id);
    if (!relayer) {
      relayer = Relayer.create({
        id: relayer_id,
        networks: `{${network}}` as unknown as SupportedChain[], // Workaround to avoid postgres issues when inserting an array of enums. See https://stackoverflow.com/questions/18234946/postgresql-insert-into-an-array-of-enums
        postRequestsHandled: BigInt(0),
        totalFeesEarned: BigInt(0),
      });
    }

    relayer.postRequestsHandled += BigInt(1);
    relayer = RelayerService.updateRelayerNetworksList(relayer, network);

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
        transfer.network,
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
    network: SupportedChain,
  ): Relayer {
    const networks = (relayer.networks as unknown as string)
      .slice(1, -1)
      .split(", ");
    if (!networks.includes(network)) {
      networks.push(network);
    }

    const networks_enum_str = networks.join(", ");
    relayer.networks = `{${networks_enum_str}}` as unknown as SupportedChain[];

    return relayer;
  }
}
