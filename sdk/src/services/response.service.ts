import { solidityKeccak256 } from "ethers/lib/utils";
import { Request, Response, ResponseStatusMetadata, Status } from "../types";

export interface ICreateResponseArgs {
  chain: string;
  commitment: string;
  response_message?: string | undefined;
  responseTimeoutTimestamp?: bigint | undefined;
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
  blockTimestamp: bigint;
  chain: string;
}

const RESPONSE_STATUS_WEIGHTS = {
  [Status.SOURCE]: 1,
  [Status.MESSAGE_RELAYED]: 2,
  [Status.DEST]: 3,
  [Status.TIMED_OUT]: 4,
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

    if (typeof response === "undefined") {
      response = Response.create({
        id: commitment,
        commitment,
        chain,
        response_message,
        requestId: request?.id,
        status,
        responseTimeoutTimestamp,
        sourceTransactionHash: transactionHash,
        hyperbridgeTransactionHash: undefined,
        destinationTransactionHash: undefined,
      });

      await response.save();

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
      chain,
    } = args;

    let response = await Response.get(commitment);

    if (response) {
      if (
        RESPONSE_STATUS_WEIGHTS[status] >
        RESPONSE_STATUS_WEIGHTS[response.status]
      ) {
        response.status = status;

        switch (status) {
          case Status.MESSAGE_RELAYED:
            response.hyperbridgeTransactionHash = transactionHash;
            break;
          case Status.DEST:
            response.destinationTransactionHash = transactionHash;
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
    let hash = solidityKeccak256(
      [
        "bytes",
        "bytes",
        "uint64",
        "uint64",
        "bytes",
        "bytes",
        "bytes",
        "bytes",
        "uint64",
      ],
      [
        source,
        dest,
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
}
