import { solidityKeccak256 } from "ethers/lib/utils";
import { Request, ResponseStatus, Response, SupportedChain } from "../types";

export interface ICreateResponseArgs {
  chain: SupportedChain;
  commitment: string;
  response_message: string;
  status: ResponseStatus;
  responseTimeoutTimestamp: bigint;
  request: Request;
  blockNumber: string;
  transactionHash: string;
  blockTimestamp: bigint;
}

export interface IUpdateResponseStatusArgs {
  commitment: string;
  status: ResponseStatus;
  blockNumber: string;
  transactionHash: string;
  blockTimestamp: bigint;
  chain: SupportedChain;
}

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
      blockTimestamp,
      transactionHash,
    } = args;
    let response = await Response.get(commitment);

    if (typeof response === "undefined") {
      response = Response.create({
        id: commitment,
        chain,
        response_message,
        requestId: request.id,
        status,
        responseTimeoutTimestamp,
        statusMetadata: [
          {
            status,
            chain: chain,
            timestamp: blockTimestamp,
            blockNumber,
            transactionHash,
          },
        ],
      });

      await response.save();
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
      blockTimestamp,
      status,
      transactionHash,
      chain,
    } = args;

    let response = await Response.get(commitment);

    if (response) {
      response.status = status;
      response.statusMetadata.push({
        blockNumber,
        chain,
        status,
        timestamp: blockTimestamp,
        transactionHash,
      });
    } else {
      logger.error(
        `Attempted to update status of non-existent response with commitment: ${commitment} in transaction: ${transactionHash}`,
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
    responseTimeoutTimestamp: bigint,
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
      ],
    );
    return hash;
  }
}
