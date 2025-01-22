import { solidityKeccak256 } from 'ethers/lib/utils';
import { Status } from '../types/enums';
import { Request, RequestStatusMetadata } from '../types/models';

export interface ICreateRequestArgs {
 chain: string;
 commitment: string;
 body?: string | undefined;
 dest?: string | undefined;
 fee?: bigint | undefined;
 from?: string | undefined;
 nonce?: bigint | undefined;
 source?: string | undefined;
 timeoutTimestamp?: bigint | undefined;
 to?: string | undefined;
 status: Status;
 blockNumber: string;
 blockHash: string;
 transactionHash: string;
 blockTimestamp: bigint;
}

export interface IUpdateRequestStatusArgs {
 commitment: string;
 status: Status;
 blockNumber: string;
 blockHash: string;
 transactionHash: string;
 blockTimestamp: bigint;
 chain: string;
}

const REQUEST_STATUS_WEIGHTS = {
 [Status.SOURCE]: 1,
 [Status.MESSAGE_RELAYED]: 2,
 [Status.DEST]: 3,
 [Status.TIMED_OUT]: 4,
};

export class RequestService {
 /**
  * Finds a request enitity and creates a new one if it doesn't exist
  */
 static async findOrCreate(args: ICreateRequestArgs): Promise<Request> {
  const {
   chain,
   commitment,
   body,
   dest,
   fee,
   from,
   nonce,
   source,
   status,
   timeoutTimestamp,
   to,
   blockNumber,
   blockHash,
   transactionHash,
   blockTimestamp,
  } = args;
  let request = await Request.get(commitment);

  if (typeof request === 'undefined') {
   request = Request.create({
    id: commitment,
    chain,
    body: body || '',
    dest: dest || '',
    fee: fee || BigInt(0),
    from: from || '',
    nonce: nonce || BigInt(0),
    source: source || '',
    status,
    timeoutTimestamp: timeoutTimestamp || BigInt(0),
    to: to || '',
    sourceTransactionHash: transactionHash,
    hyperbridgeTransactionHash: '',
    destinationTransactionHash: '',
    commitment,
   });

   await request.save();

   let requestStatusMetadata = RequestStatusMetadata.create({
    id: `${commitment}.${status}`,
    requestId: commitment,
    status,
    chain,
    timestamp: blockTimestamp,
    blockNumber,
    blockHash,
    transactionHash,
   });

   await requestStatusMetadata.save();
  }

  return request;
 }

 /**
  * Update the status of a request
  * Also adds a new entry to the request status metadata
  */
 static async updateStatus(args: IUpdateRequestStatusArgs): Promise<void> {
  const {
   commitment,
   blockNumber,
   blockHash,
   blockTimestamp,
   status,
   transactionHash,
   chain,
  } = args;

  let request = await Request.get(commitment);

  if (request) {
   if (
    REQUEST_STATUS_WEIGHTS[status] > REQUEST_STATUS_WEIGHTS[request.status]
   ) {
    logger.info(
     `Updating Request Status: ${JSON.stringify({
      new_status: status,
      old_status: request.status,
      is_true:
       REQUEST_STATUS_WEIGHTS[status] > REQUEST_STATUS_WEIGHTS[request.status],
     })}`
    );

    request.status = status;

    switch (status) {
     case Status.MESSAGE_RELAYED:
      request.hyperbridgeTransactionHash = transactionHash;
      break;
     case Status.DEST:
      request.destinationTransactionHash = transactionHash;
      break;
    }

    await request.save();
   }

   let requestStatusMetadata = RequestStatusMetadata.create({
    id: `${commitment}.${status}`,
    requestId: commitment,
    status,
    chain,
    timestamp: blockTimestamp,
    blockNumber,
    blockHash,
    transactionHash,
   });

   await requestStatusMetadata.save();
  } else {
   // Create new request and request status metadata
   await this.findOrCreate({
    commitment,
    chain,
    body: undefined,
    dest: undefined,
    fee: undefined,
    from: undefined,
    nonce: undefined,
    source: undefined,
    timeoutTimestamp: undefined,
    to: undefined,
    blockNumber,
    blockHash,
    blockTimestamp,
    status,
    transactionHash,
   });

   logger.info(
    `Created new request while attempting request update with details ${JSON.stringify(
     { commitment, transactionHash, status }
    )}`
   );
  }
 }

 /**
  * Compute the request commitment
  */
 static computeRequestCommitment(
  source: string,
  dest: string,
  nonce: bigint,
  timeoutTimestamp: bigint,
  from: string,
  to: string,
  body: string
 ): string {
  let hash = solidityKeccak256(
   ['bytes', 'bytes', 'uint64', 'uint64', 'bytes', 'bytes', 'bytes'],
   [source, dest, nonce, timeoutTimestamp, from, to, body]
  );
  return hash;
 }

 /**
  * Find requests by source transaction hash
  */
 static async findBySourceTransactionHash(sourceTransactionHash: string) {
  return Request.getBySourceTransactionHash(sourceTransactionHash, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by hyperbridge transaction hash
  */
 static async findByHyperbridgeTransactionHash(
  hyperbridgeTransactionHash: string
 ) {
  return Request.getByHyperbridgeTransactionHash(hyperbridgeTransactionHash, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by destination transaction hash
  */
 static async findByDestinationTransactionHash(
  destinationTransactionHash: string
 ) {
  return Request.getByDestinationTransactionHash(destinationTransactionHash, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by source chain
  */
 static async findBySource(source: string) {
  return Request.getBySource(source, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by destination chain
  */
 static async findByDest(dest: string) {
  return Request.getByDest(dest, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by sender address
  */
 static async findByFrom(from: string) {
  return Request.getByFrom(from, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by recipient address
  */
 static async findByTo(to: string) {
  return Request.getByTo(to, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by nonce
  */
 static async findByNonce(nonce: bigint) {
  return Request.getByNonce(nonce, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by status
  */
 static async findByStatus(status: Status) {
  return Request.getByStatus(status, {
   orderBy: 'nonce',
   limit: -1,
  });
 }

 /**
  * Find requests by fee amount
  */
 static async findByFee(fee: bigint) {
  return Request.getByFee(fee, {
   orderBy: 'nonce',
   limit: -1,
  });
 }
}
