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

 /**
  * Get transfers by amount
  */
 static async getByAmount(amount: bigint) {
  return Transfer.getByAmount(amount, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Get transfers by sender address
  */
 static async getByFrom(from: string) {
  return Transfer.getByFrom(from, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Get transfers by recipient address
  */
 static async getByTo(to: string) {
  return Transfer.getByTo(to, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Get transfers by chain
  */
 static async getByChain(chain: string) {
  return Transfer.getByChain(chain, {
   orderBy: 'id',
   limit: -1,
  });
 }
}
