import { Event, StateMachineUpdateEvent } from "../types/models";
import { EventType, SupportedChain } from "../types";

interface IEvmHostEventArgs {
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

// Arguments to functions that create StateMachineUpdated events
export interface ICreateStateMachineUpdatedEventArgs extends IEvmHostEventArgs {
  stateMachineId: string;
  height: bigint;
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

  /**
   * Create a new EVM Host StateMachineUpdated event entity
   */
  static async createStateMachineUpdatedEvent(
    args: ICreateStateMachineUpdatedEventArgs,
    chain: SupportedChain,
  ): Promise<void> {
    const {
      blockHash,
      blockNumber,
      transactionHash,
      transactionIndex,
      timestamp,
      stateMachineId,
      height,
    } = args;

    const event = StateMachineUpdateEvent.create({
      id: `${stateMachineId}_${height}`,
      stateMachineId,
      height,
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
