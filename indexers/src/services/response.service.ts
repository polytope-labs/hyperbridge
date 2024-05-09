import { solidityKeccak256 } from "ethers/lib/utils";
import {
  Request,
  ResponseStatus,
  Response,
  ResponseStatusMetadata,
  SupportedChain,
} from "../types";

export interface ICreateResponseArgs {
  chain: SupportedChain;
  commitment: string;
  response_message: string;
  status: ResponseStatus;
  responseTimeoutTimestamp: bigint;
  request: Request;
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
      });

      await response.save();
    }

    return response;
  }

  /**
   * Finds a response metadata enitity and creates a new one if it doesn't exist
   */
  static async findOrCreateMetadata(
    response_commitment: string,
    sourceBlockNumber?: bigint,
    sourceBlockTransaction?: string,
    messageRelayedTransactionHash?: string,
    destFinalizedTransactionHash?: string,
    deliveryTransactionHash?: string,
  ): Promise<ResponseStatusMetadata> {
    let responseMetadata =
      await ResponseStatusMetadata.get(response_commitment);

    if (typeof responseMetadata === "undefined") {
      responseMetadata = ResponseStatusMetadata.create({
        id: response_commitment,
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

      await responseMetadata.save();
    }

    return responseMetadata;
  }

  /**
   * Update the status of a response and response metadata
   */
  static async updateResponseStatus(
    response_commitment: string,
    status: ResponseStatus,
    sourceBlockNumber?: bigint,
    sourceBlockTransaction?: string,
    messageRelayedTransactionHash?: string,
    destFinalizedTransactionHash?: string,
    deliveryTransactionHash?: string,
  ): Promise<void> {
    let response = await Response.get(response_commitment);
    let responseMetadata =
      await ResponseService.findOrCreateMetadata(response_commitment);

    if (response) {
      response.status = status;
      sourceBlockNumber
        ? (responseMetadata.sourceBlockNumber = sourceBlockNumber)
        : "";
      sourceBlockTransaction
        ? (responseMetadata.sourceBlockTransaction = sourceBlockTransaction)
        : "";
      messageRelayedTransactionHash
        ? (responseMetadata.messageRelayedTransactionHash =
            messageRelayedTransactionHash)
        : "";
      destFinalizedTransactionHash
        ? (responseMetadata.destFinalizedTransactionHash =
            destFinalizedTransactionHash)
        : "";
      deliveryTransactionHash
        ? (responseMetadata.deliveryTransactionHash = deliveryTransactionHash)
        : "";

      await response.save();
      await responseMetadata.save();
    } else {
      logger.info(
        `Attempted to update status of non-existent response with commitment: ${response_commitment} in transaction: ${sourceBlockTransaction}`,
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
