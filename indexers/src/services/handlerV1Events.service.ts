import { SupportedChain } from "../types";
import { ICreateEvmHostEventArgs } from "./evmHostEvents.service";
import { Event } from "../types/models";

export interface IHandleStateMachineUpdatedEventArgs
  extends ICreateEvmHostEventArgs {
  metadata: {
    stateMachineId: bigint;
    height: bigint;
  };
}

export class HandlerV1EventsService {
  /**
   * Handles the StateMachineUpdated event from HandlerV1 contract
   */
  static async createEvent(
    args: IHandleStateMachineUpdatedEventArgs,
    chain: SupportedChain,
  ): Promise<void> {
    const {
      type,
      timestamp,
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      metadata: { stateMachineId, height },
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
      metadata: { stateMachineId, height },
    });

    await event.save();
  }
}
