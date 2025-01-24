import { SubstrateEvent } from '@subql/types';

/**
 * Get the StateMachineID parsing the stringified object which substrate provides
 */
export const extractStateMachineIdFromSubstrateEventData = (
 substrateStateMachineId: string
): string | undefined => {
 try {
  const parsed = JSON.parse(substrateStateMachineId);
  let stateId;

  // Handle array format with direct objects
  if (Array.isArray(parsed)) {
   // Find the object containing stateId or ethereum/bsc keys
   const stateObject = parsed.find(
    (item) => item?.stateId || item?.ethereum || item?.bsc
   );

   if (!stateObject) return undefined;

   // Extract stateId from different formats
   stateId = stateObject.stateId || stateObject;
  } else {
   // Handle object format
   stateId = parsed.stateId;
  }

  if (!stateId) {
   throw new Error(
    `StateId not present in stateMachineId: ${substrateStateMachineId}`
   );
  }

  // Extract key and value
  let main_key = '';
  let value = '';

  Object.entries(stateId).forEach(([key, val]) => {
   main_key = key.toUpperCase();
   value = val === null ? '' : String(val);
  });

  switch (main_key) {
   case 'EVM':
   case 'ETHEREUM':
    return 'EVM-'.concat(value);
   case 'POLKADOT':
    return 'POLKADOT-'.concat(value);
   case 'KUSAMA':
    return 'KUSAMA-'.concat(value);
   case 'BEEFY':
    return 'BEEFY-'.concat(value);
   case 'GRANDPA':
    return 'GRANDPA-'.concat(value);
   case 'BSC':
    return 'BSC-'.concat(value);
   default:
    throw new Error(
     `Unknown state machine ID ${main_key} encountered in extractStateMachineIdFromSubstrateEventData. `
    );
  }
 } catch (error) {
  logger.error(error);
  return undefined;
 }
};

/**
 * Get the chainId from the event
 */
export function getChainIdFromEvent(event: SubstrateEvent): string {
 const chainId =
  event.block.block.header.parentHash.toString().length > 0
   ? event.block.block.header.parentHash.toString() // Parachain
   : event.block.block.header.hash.toString(); // Standalone chain

 return chainId;
}

/**
 * Error class for substrate indexing errors
 */
export class SubstrateIndexingError extends Error {
 constructor(
  message: string,
  public chainId: string,
  public blockNumber?: number,
  public eventMethod?: string
 ) {
  super(message);
  this.name = 'SubstrateIndexingError';
 }
}

/**
 * Error class for state machine errors
 */
export class StateMachineError extends SubstrateIndexingError {
 constructor(message: string, chainId: string, blockNumber?: number) {
  super(message, chainId, blockNumber);
  this.name = 'StateMachineError';
 }
}

/**
 * Error class for asset events
 */
export class AssetEventError extends SubstrateIndexingError {
 constructor(message: string, chainId: string, blockNumber?: number) {
  super(message, chainId, blockNumber);
  this.name = 'AssetEventError';
 }
}

export class SubstrateEventValidator {
 /**
  * Validate state machine event data
  */
 static validateStateMachineEvent(event: SubstrateEvent): boolean {
  const { data, method } = event.event;

  switch (method) {
   case 'StateMachineUpdated':
    return (
     data.length >= 1 &&
     typeof Number(data[0].toString()) === 'function' &&
     !isNaN(Number(data[0].toString()))
    );

   case 'MessageProcessed':
    return (
     data.length >= 3 &&
     typeof data[0].toString === 'function' &&
     typeof data[1].toString === 'function' &&
     typeof Number(data[2].toString()) === 'function'
    );

   default:
    return false;
  }
 }

 /**
  * Validate asset event data
  */
 static validateAssetEvent(event: SubstrateEvent): boolean {
  const { data, method } = event.event;

  switch (method) {
   case 'AssetTransferred':
    return (
     data.length >= 5 &&
     typeof data[0].toString === 'function' && // commitment
     typeof data[1].toString === 'function' && // amount
     typeof data[2].toString === 'function' && // assetId
     typeof data[3].toString === 'function' && // to address
     typeof data[4].toString === 'function' // from address
    );

   default:
    return false;
  }
 }

 /**
  * Validate chain metadata
  */
 static validateChainMetadata(
  chainId: string,
  stateMachineId: string
 ): boolean {
  return (
   typeof chainId === 'string' &&
   chainId.length > 0 &&
   typeof stateMachineId === 'string' &&
   stateMachineId.length > 0
  );
 }
}
