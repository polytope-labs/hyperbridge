import assert from "assert";
import {
  ResponseStatus,
  SupportedChain,
  Request,
  RequestStatus,
} from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { PostResponseEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { ResponseService } from "../../../services/response.service";
import { RequestService } from "../../../services/request.service";

/**
 * Handles the PostResponse event from Evm Hosts
 */
export async function handlePostResponseEvent(
  event: PostResponseEventLog,
): Promise<void> {
  assert(event.args, "No handlePostResponseEvent args");
  logger.info("Handling PostResponse event");

  const { transaction, blockNumber, transactionHash, args } = event;
  let {
    data,
    dest,
    fee,
    from,
    nonce,
    source,
    timeoutTimestamp,
    to,
    response,
    resTimeoutTimestamp,
  } = args;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event);

  // Compute the response commitment
  let response_commitment = ResponseService.computeResponseCommitment(
    source,
    dest,
    BigInt(nonce.toString()),
    BigInt(timeoutTimestamp.toString()),
    from,
    to,
    data,
    response,
    BigInt(resTimeoutTimestamp.toString()),
  );

  // Compute the request commitment
  let request_commitment = RequestService.computeRequestCommitment(
    source,
    dest,
    BigInt(nonce.toString()),
    BigInt(timeoutTimestamp.toString()),
    from,
    to,
    data,
  );

  let request = await Request.get(request_commitment);
  if (typeof request === "undefined") {
    request = await RequestService.findOrCreate({
      chain,
      commitment: request_commitment,
      data,
      dest,
      fee: BigInt(fee.toString()),
      from,
      nonce: BigInt(nonce.toString()),
      source,
      status: RequestStatus.SOURCE,
      timeoutTimestamp: BigInt(timeoutTimestamp.toString()),
      to,
    });
  }

  // Create the response entity
  await ResponseService.findOrCreate({
    chain,
    commitment: response_commitment,
    responseTimeoutTimestamp: BigInt(resTimeoutTimestamp.toString()),
    response_message: response,
    status: ResponseStatus.SOURCE,
    request,
  });

  await ResponseService.updateResponseStatus(
    response_commitment,
    ResponseStatus.SOURCE,
    BigInt(blockNumber),
    transactionHash,
  );
}
