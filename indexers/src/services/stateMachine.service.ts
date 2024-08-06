import { StateMachineUpdateEvent } from "../types";
import { IEvmHostEventArgs } from "./evmHostEvents.service";

// Arguments to functions that create StateMachineUpdated events
export interface ICreateStateMachineUpdatedEventArgs extends IEvmHostEventArgs {
  stateMachineId: string;
  height: number;
}

export class StateMachineService {
  /**
   * Create a new Evm Host StateMachineUpdated event entity
   */
  static async createEvmStateMachineUpdatedEvent(
    args: ICreateStateMachineUpdatedEventArgs,
    chain: string
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
      id: `${chain}_${transactionHash}_${stateMachineId}_${height}`,
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

  /**
   * Create a new Hyperbridge StateMachineUpdated event entity
   */
  static async createHyperbridgeStateMachineUpdatedEvent(
    args: ICreateStateMachineUpdatedEventArgs,
    chain: string
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
      id: `${stateMachineId}-${transactionHash}-${height}`,
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
