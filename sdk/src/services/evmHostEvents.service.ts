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

 /**
  * Get events by type
  */
 static async getEventsByType(type: EventType) {
  return Event.getByType(type, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }

 /**
  * Get events by block hash
  */
 static async getEventsByBlockHash(blockHash: string) {
  return Event.getByBlockHash(blockHash, {
   orderBy: 'transactionIndex',
   limit: -1,
  });
 }

 /**
  * Get events by block number
  */
 static async getEventsByBlockNumber(blockNumber: bigint) {
  return Event.getByBlockNumber(blockNumber, {
   orderBy: 'transactionIndex',
   limit: -1,
  });
 }

 /**
  * Get events by transaction hash
  */
 static async getEventsByTransactionHash(transactionHash: string) {
  return Event.getByTransactionHash(transactionHash, {
   orderBy: 'transactionIndex',
   limit: -1,
  });
 }

 /**
  * Get events by transaction index
  */
 static async getEventsByTransactionIndex(transactionIndex: bigint) {
  return Event.getByTransactionIndex(transactionIndex, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }

 /**
  * Get events by destination address
  */
 static async getEventsByDestination(destination: string) {
  return Event.getByDestination(destination, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }

 /**
  * Get events by creation date
  */
 static async getEventsByCreatedAt(createdAt: Date) {
  return Event.getByCreatedAt(createdAt, {
   orderBy: 'blockNumber',
   limit: -1,
  });
 }
}
