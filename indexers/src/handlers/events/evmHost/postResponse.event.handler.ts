import assert from "assert";
import {
  Status,
  SupportedChain,
  Request,
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

  const { transaction, blockNumber, transactionHash, args, block } = event;
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

  logger.info(
    `Handling PostResponse Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

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
    logger.error(
      `Error handling PostResponseEvent because request with commitment: ${request_commitment} was not found`,
    );
    return;
  }

  // Create the response entity
  await ResponseService.findOrCreate({
    chain,
    commitment: response_commitment,
    responseTimeoutTimestamp: BigInt(resTimeoutTimestamp.toString()),
    response_message: response,
    status: Status.SOURCE,
    request,
    blockNumber: blockNumber.toString(),
    blockHash: block.hash,
    transactionHash,
    blockTimestamp: block.timestamp,
  });
}
