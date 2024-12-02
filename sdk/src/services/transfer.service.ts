import { BigNumber } from "ethers";
import { Transfer } from "../types/models";

// Argument for storing transfer events
export interface IStoreTranferArgs {
  from: string;
  to: string;
  value: BigNumber;
  transactionHash: string;
  chain: string;
}

export class TransferService {
  /**
   * Increment the number of post requests handled by a relayer
   */
  static async storeTransfer(arg: IStoreTranferArgs): Promise<Transfer> {
    const { from, to, value, transactionHash, chain } = arg;
    let transfer = Transfer.create({
      id: transactionHash,
      amount: BigInt(value.toString()),
      from,
      to,
      chain,
    });

    await transfer.save();
    return transfer;
  }
}
