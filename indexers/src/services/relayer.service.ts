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
        network,
        postRequestsHandled: BigInt(0),
        totalFeesEarned: BigInt(0),
      });
    }

    relayer.postRequestsHandled += BigInt(1);
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
      await relayer.save();
    }
  }
}
