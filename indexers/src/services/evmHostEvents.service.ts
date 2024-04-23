import { Event } from "../types/models";
import { EventType, SupportedChain } from "../types";

// Arguments to functions that create EvmHost events
export interface ICreateEvmHostEventArgs {
  blockHash: string;
  blockNumber: number;
  transactionHash: string;
  transactionIndex: number;
  timestamp: number;
  type: EventType;
}

export class EvmHostEventsService {
  /**
   * Create a new EVM Host event entity
   */
  static async createEvent(
    args: ICreateEvmHostEventArgs,
    chain: SupportedChain,
  ): Promise<void> {
    const {
      blockHash,
      blockNumber,
      transactionHash,
      transactionIndex,
      timestamp,
      type,
    } = args;

    const event = Event.create({
      id: transactionHash,
      type,
      chain,
      transactionHash,
      transactionIndex: BigInt(transactionIndex),
      blockHash,
      blockNumber: BigInt(blockNumber),
      createdAt: new Date(timestamp * 1000),
    });

    await event.save();
  }
}
