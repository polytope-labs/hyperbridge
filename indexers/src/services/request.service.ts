import { RequestStatus } from "../types/enums";
import { solidityKeccak256 } from "ethers/lib/utils";
import { Request, RequestStatusMetadata } from "../types/models";

export class RequestService {
  /**
   * Finds a request enitity and creates a new one if it doesn't exist
   */
  static async findOrCreate(
    request_commitment: string,
    data?: string,
    dest?: string,
    fee?: bigint,
    from?: string,
    nonce?: bigint,
    source?: string,
    status?: RequestStatus,
    timeoutTimestamp?: bigint,
    to?: string,
  ): Promise<Request> {
    let request = await Request.get(request_commitment);

    if (typeof request === "undefined") {
      if (
        typeof status === "undefined" ||
        typeof source === "undefined" ||
        typeof to === "undefined" ||
        typeof from === "undefined" ||
        typeof dest === "undefined" ||
        typeof fee === "undefined" ||
        typeof nonce === "undefined" ||
        typeof timeoutTimestamp === "undefined" ||
        typeof data === "undefined"
      ) {
        throw new Error("Request creation requires all data");
      }
      request = Request.create({
        id: request_commitment,
        data: data ? data : "",
        dest: dest ? dest : "",
        fee: fee ? fee : BigInt(0),
        from: from ? from : "",
        nonce: nonce ? nonce : BigInt(0),
        source: source ? source : "",
        status: status ? status : RequestStatus.SOURCE,
        timeoutTimestamp: timeoutTimestamp ? timeoutTimestamp : BigInt(0),
        to: to ? to : "",
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
