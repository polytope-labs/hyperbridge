import { Event } from "../types/models";
import { EventType } from "../types";

export interface IEvmHostEventArgs {
  blockHash: string;
  blockNumber: number;
  transactionHash: string;
  transactionIndex: number;
  timestamp: number;
}

// Arguments to functions that create EvmHost events
export interface ICreateEvmHostEventArgs extends IEvmHostEventArgs {
  type: EventType;
  commitment: string;
  data: string;
  dest?: string;
}

export class EvmHostEventsService {
  /**
   * Create a new EVM Host event entity
   */
  static async createEvent(
    args: ICreateEvmHostEventArgs,
    chain: string
  ): Promise<void> {
    const {
      commitment,
      blockHash,
      blockNumber,
      transactionHash,
      transactionIndex,
      timestamp,
      type,
      data,
      dest,
    } = args;

    const event = Event.create({
      id: commitment,
      data,
      type,
      chain,
      transactionHash,
      destination: dest,
      transactionIndex: BigInt(transactionIndex),
      blockHash,
      blockNumber: BigInt(blockNumber),
      createdAt: new Date(timestamp * 1000),
    });

    await event.save();
  }
}
