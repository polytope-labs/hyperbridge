import { RequestStatus, SupportedChain } from "../types/enums";
import { solidityKeccak256 } from "ethers/lib/utils";
import { Request, RequestStatusMetadata } from "../types/models";

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
      });

      await request.save();
    }

    return request;
  }

  /**
   * Finds a request metadata enitity and creates a new one if it doesn't exist
   */
  static async findOrCreateMetadata(
    request_commitment: string,
    sourceBlockNumber?: bigint,
    sourceBlockTransaction?: string,
    messageRelayedTransactionHash?: string,
    destFinalizedTransactionHash?: string,
    deliveryTransactionHash?: string,
  ): Promise<RequestStatusMetadata> {
    let requestMetadata = await RequestStatusMetadata.get(request_commitment);

    if (typeof requestMetadata === "undefined") {
      requestMetadata = RequestStatusMetadata.create({
        id: request_commitment,
        sourceBlockNumber: sourceBlockNumber ? sourceBlockNumber : BigInt(0),
        sourceBlockTransaction: sourceBlockTransaction
          ? sourceBlockTransaction
          : "",
        messageRelayedTransactionHash: messageRelayedTransactionHash
          ? messageRelayedTransactionHash
          : "",
        destFinalizedTransactionHash: destFinalizedTransactionHash
          ? destFinalizedTransactionHash
          : "",
        deliveryTransactionHash: deliveryTransactionHash
          ? deliveryTransactionHash
          : "",
      });

      await requestMetadata.save();
    }

    return requestMetadata;
  }

  /**
   * Update the status of a request and request metadata
   */
  static async updateRequestStatus(
    request_commitment: string,
    status: RequestStatus,
    sourceBlockNumber?: bigint,
    sourceBlockTransaction?: string,
    messageRelayedTransactionHash?: string,
    destFinalizedTransactionHash?: string,
    deliveryTransactionHash?: string,
  ): Promise<void> {
    let request = await Request.get(request_commitment);
    let requestMetadata =
      await RequestService.findOrCreateMetadata(request_commitment);

    if (request) {
      request.status = status;
      sourceBlockNumber
        ? (requestMetadata.sourceBlockNumber = sourceBlockNumber)
        : "";
      sourceBlockTransaction
        ? (requestMetadata.sourceBlockTransaction = sourceBlockTransaction)
        : "";
      messageRelayedTransactionHash
        ? (requestMetadata.messageRelayedTransactionHash =
            messageRelayedTransactionHash)
        : "";
      destFinalizedTransactionHash
        ? (requestMetadata.destFinalizedTransactionHash =
            destFinalizedTransactionHash)
        : "";
      deliveryTransactionHash
        ? (requestMetadata.deliveryTransactionHash = deliveryTransactionHash)
        : "";

      await request.save();
      await requestMetadata.save();
    } else {
      logger.info(
        `Attempted to update status of non-existent request with commitment: ${request_commitment} in transaction: ${sourceBlockTransaction}`,
      );
    }
  }

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
