import { RequestStatus, SupportedChain } from "../types/enums";
import { solidityKeccak256 } from "ethers/lib/utils";
import { Request } from "../types/models";

export interface ICreateRequestArgs {
  chain: SupportedChain;
  commitment: string;
  data: string;
  dest: string;
  fee: bigint;
  from: string;
  nonce: bigint;
  source: string;
  status: RequestStatus;
  timeoutTimestamp: bigint;
  to: string;
  blockNumber: string;
  transactionHash: string;
  blockTimestamp: bigint;
}

export interface IUpdateRequestStatusArgs {
  commitment: string;
  status: RequestStatus;
  blockNumber: string;
  transactionHash: string;
  blockTimestamp: bigint;
  chain: SupportedChain;
}

export class RequestService {
  /**
   * Finds a request enitity and creates a new one if it doesn't exist
   */
  static async findOrCreate(args: ICreateRequestArgs): Promise<Request> {
    const {
      chain,
      commitment,
      data,
      dest,
      fee,
      from,
      nonce,
      source,
      status,
      timeoutTimestamp,
      to,
      blockNumber,
      transactionHash,
      blockTimestamp,
    } = args;
    let request = await Request.get(commitment);

    if (typeof request === "undefined") {
      request = Request.create({
        id: commitment,
        chain,
        data,
        dest,
        fee,
        from,
        nonce,
        source,
        status,
        timeoutTimestamp,
        to,
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

      await request.save();
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
      blockTimestamp,
      status,
      transactionHash,
      chain,
    } = args;

    let request = await Request.get(commitment);

    if (request) {
      request.status = status;
      request.statusMetadata.push({
        blockNumber,
        chain,
        status,
        timestamp: blockTimestamp,
        transactionHash,
      });
    } else {
      logger.error(
        `Attempted to update status of non-existent request with commitment: ${commitment} in transaction: ${transactionHash}`,
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
    body: string,
  ): string {
    let hash = solidityKeccak256(
      ["bytes", "bytes", "uint64", "uint64", "bytes", "bytes", "bytes"],
      [source, dest, nonce, timeoutTimestamp, from, to, body],
    );
    return hash;
  }
}
