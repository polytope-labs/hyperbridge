import { StateMachineUpdateEvent } from '../types';
import { IEvmHostEventArgs } from './evmHostEvents.service';

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

 /**
  * Get updates by state machine ID
  */
 static async getByStateMachineId(stateMachineId: string) {
  return StateMachineUpdateEvent.getByStateMachineId(stateMachineId, {
   orderBy: 'height',
   limit: -1,
  });
 }

 /**
  * Get updates by height
  */
 static async getByHeight(height: number) {
  return StateMachineUpdateEvent.getByHeight(height, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }

 /**
  * Get updates by block number
  */
 static async getByBlockNumber(blockNumber: bigint) {
  return StateMachineUpdateEvent.getByBlockNumber(blockNumber, {
   orderBy: 'transactionIndex',
   limit: -1,
  });
 }

 /**
  * Get updates by creation date
  */
 static async getByCreatedAt(createdAt: Date) {
  return StateMachineUpdateEvent.getByCreatedAt(createdAt, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }
}
