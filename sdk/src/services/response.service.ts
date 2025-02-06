import { solidityKeccak256 } from 'ethers/lib/utils';
import { Request, Response, ResponseStatusMetadata, Status } from '../types';
import { ethers } from 'ethers';

export interface ICreateResponseArgs {
 chain: string;
 commitment: string;
 response_message?: string | undefined;
 responseTimeoutTimestamp?: bigint | undefined;
 destination_timeout_transaction_hash?: string | undefined;
 hyperbridge_timeout_transaction_hash?: string | undefined;
 request?: Request | undefined;
 status: Status;
 blockNumber: string;
 blockHash: string;
 transactionHash: string;
 blockTimestamp: bigint;
}

export interface IUpdateResponseStatusArgs {
 commitment: string;
 status: Status;
 blockNumber: string;
 blockHash: string;
 transactionHash: string;
 timeoutHash?: string;
 blockTimestamp: bigint;
 chain: string;
}

const RESPONSE_STATUS_WEIGHTS = {
 [Status.SOURCE]: 1,
 [Status.HYPERBRIDGE_DELIVERED]: 2,
 [Status.DESTINATION]: 3,
 [Status.HYPERBRIDGE_TIMED_OUT]: 4,
 [Status.TIMED_OUT]: 5,
};

export class ResponseService {
 /**
  * Finds a response enitity and creates a new one if it doesn't exist
  */
 static async findOrCreate(args: ICreateResponseArgs): Promise<Response> {
  const {
   chain,
   commitment,
   request,
   response_message,
   responseTimeoutTimestamp,
   status,
   blockNumber,
   blockHash,
   blockTimestamp,
   transactionHash,
  } = args;
  let response = await Response.get(commitment);

  logger.info(
   `Creating PostResponse Event: ${JSON.stringify({
    commitment,
    transactionHash,
    status,
   })}`
  );

  if (typeof response === 'undefined') {
   response = Response.create({
    id: commitment,
    commitment,
    chain,
    response_message,
    requestId: request?.id,
    status,
    responseTimeoutTimestamp,
    sourceTransactionHash: transactionHash,
    hyperbridgeTransactionHash: '',
    destinationTransactionHash: '',
    destination_timeout_transaction_hash: '',
    hyperbridge_timeout_transaction_hash: '',
   });

   await response.save();

   logger.info(
    `Created new response with details ${JSON.stringify({
     commitment,
     transactionHash,
     status,
    })}`
   );

   let responseStatusMetadata = ResponseStatusMetadata.create({
    id: `${commitment}.${status}`,
    responseId: commitment,
    status,
    chain,
    timestamp: blockTimestamp,
    blockNumber,
    blockHash,
    transactionHash,
   });

   await responseStatusMetadata.save();
  }

  return response;
 }

 /**
  * Update the status of a response
  * Also adds a new entry to the response status metadata
  */
 static async updateStatus(args: IUpdateResponseStatusArgs): Promise<void> {
  const {
   commitment,
   blockNumber,
   blockHash,
   blockTimestamp,
   status,
   transactionHash,
   timeoutHash,
   chain,
  } = args;

  let response = await Response.get(commitment);

  if (response) {
   if (
    RESPONSE_STATUS_WEIGHTS[status] > RESPONSE_STATUS_WEIGHTS[response.status]
   ) {
    response.status = status;

    switch (status) {
     case Status.HYPERBRIDGE_DELIVERED:
      response.hyperbridgeTransactionHash = transactionHash;
      break;
     case Status.DESTINATION:
      response.destinationTransactionHash = transactionHash;
      break;
    }

    switch (timeoutHash) {
     case 'hyperbridge_timeout':
      response.hyperbridge_timeout_transaction_hash = transactionHash;
      break;
     case 'destination_timeout':
      response.destination_timeout_transaction_hash = transactionHash;
      break;
    }

    await response.save();
   }

   let responseStatusMetadata = ResponseStatusMetadata.create({
    id: `${commitment}.${status}`,
    responseId: commitment,
    status,
    chain,
    timestamp: blockTimestamp,
    blockNumber,
    blockHash,
    transactionHash,
   });

   await responseStatusMetadata.save();
  } else {
   await this.findOrCreate({
    chain,
    commitment,
    blockHash,
    blockNumber,
    blockTimestamp,
    status,
    transactionHash,
    request: undefined,
    responseTimeoutTimestamp: undefined,
    response_message: undefined,
    destination_timeout_transaction_hash:
     timeoutHash === 'destination_timeout' ? transactionHash : undefined,
    hyperbridge_timeout_transaction_hash:
     timeoutHash === 'hyperbridge_timeout' ? transactionHash : undefined,
   });

   logger.error(
    `Attempted to update status of non-existent response with commitment: ${commitment} in transaction: ${transactionHash}`
   );

   logger.info(
    `Created new response while attempting response update with details: ${JSON.stringify(
     { commitment, transactionHash, status }
    )}`
   );
  }
 }

 /**
  * Compute the response commitment and return the hash
  */
 static computeResponseCommitment(
  source: string,
  dest: string,
  nonce: bigint,
  timeoutTimestamp: bigint,
  from: string,
  to: string,
  body: string,
  response: string,
  responseTimeoutTimestamp: bigint
 ): string {
  logger.info(
   `Computing response commitment with details ${JSON.stringify({
    source,
    dest,
    nonce: nonce.toString(),
    timeoutTimestamp: timeoutTimestamp.toString(),
    responseTimeoutTimestamp: responseTimeoutTimestamp.toString(),
    response,
    from,
    to,
    body,
   })}`
  );

  // Convert source, dest, from, to, body to bytes
  const sourceByte = ethers.utils.toUtf8Bytes(source);
  const destByte = ethers.utils.toUtf8Bytes(dest);

  let hash = solidityKeccak256(
   [
    'bytes',
    'bytes',
    'uint64',
    'uint64',
    'bytes',
    'bytes',
    'bytes',
    'bytes',
    'uint64',
   ],
   [
    sourceByte,
    destByte,
    nonce,
    timeoutTimestamp,
    from,
    to,
    body,
    response,
    responseTimeoutTimestamp,
   ]
  );
  return hash;
 }

 /**
  * Find responses by chain
  */
 static async findByChain(chain: string) {
  return Response.getByChain(chain, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Find responses by commitment
  */
 static async findByCommitment(commitment: string) {
  return Response.getByCommitment(commitment, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Find responses by source transaction hash
  */
 static async findBySourceTransactionHash(sourceTransactionHash: string) {
  return Response.getBySourceTransactionHash(sourceTransactionHash, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Find responses by hyperbridge transaction hash
  */
 static async findByHyperbridgeTransactionHash(
  hyperbridgeTransactionHash: string
 ) {
  return Response.getByHyperbridgeTransactionHash(hyperbridgeTransactionHash, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Find responses by destination transaction hash
  */
 static async findByDestinationTransactionHash(
  destinationTransactionHash: string
 ) {
  return Response.getByDestinationTransactionHash(destinationTransactionHash, {
   orderBy: 'id',
   limit: -1,
  });
 }

 /**
  * Find responses by request ID
  */
 static async findByRequestId(requestId: string) {
  return Response.getByRequestId(requestId, {
   orderBy: 'id',
   limit: -1,
  });
 }
}
