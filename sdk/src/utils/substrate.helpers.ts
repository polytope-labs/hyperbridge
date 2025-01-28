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
    return 'EVM-'.concat(value);
   case 'POLKADOT':
    return 'POLKADOT-'.concat(value);
   case 'KUSAMA':
    return 'KUSAMA-'.concat(value);
   case 'SUBSTRATE':
    return 'SUBSTRATE-'.concat(value);
   case 'TENDERMINT':
    return 'TENDERMINT-'.concat(value);
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
    // Check data array exists and has required elements
    if (!Array.isArray(data) || data.length < 2) return false;

    // Validate first element has stateId and consensusStateId
    const stateData = data[0].toJSON();
    if (
     typeof stateData !== 'object' ||
     !stateData ||
     !('stateId' in stateData) ||
     !('consensusStateId' in stateData)
    )
     return false;

    // Validate second element is a number (height)
    const height = Number(data[1].toString());
    return !isNaN(height);

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
