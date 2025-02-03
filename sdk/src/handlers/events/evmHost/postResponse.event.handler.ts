import assert from "assert";
import { Status, Request } from "../../../types";
import { PostResponseEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { ResponseService } from "../../../services/response.service";
import { RequestService } from "../../../services/request.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the PostResponse event from Evm Hosts
 */
export async function handlePostResponseEvent(
  event: PostResponseEventLog
): Promise<void> {
  if (!event.args) return;

  const { transaction, blockNumber, transactionHash, args, block } = event;
  let {
    body,
    dest,
    fee,
    from,
    nonce,
    source,
    timeoutTimestamp,
    to,
    response,
    responseTimeoutTimestamp,
  } = args;

  logger.info(
    `Handling PostResponse Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event);

  // Compute the response commitment
  let response_commitment = ResponseService.computeResponseCommitment(
    source,
    dest,
    BigInt(nonce.toString()),
    BigInt(timeoutTimestamp.toString()),
    from,
    to,
    body,
    response,
    BigInt(responseTimeoutTimestamp.toString())
  );

  // Compute the request commitment
  let request_commitment = RequestService.computeRequestCommitment(
    source,
    dest,
    BigInt(nonce.toString()),
    BigInt(timeoutTimestamp.toString()),
    from,
    to,
    body
  );

  let request = await Request.get(request_commitment);

  if (typeof request === "undefined") {
    logger.error(
      `Error handling PostResponseEvent because request with commitment: ${request_commitment} was not found`
    );
    return;
  }

  // Create the response entity
  await ResponseService.findOrCreate({
    chain,
    commitment: response_commitment,
    responseTimeoutTimestamp: BigInt(responseTimeoutTimestamp.toString()),
    response_message: response,
    status: Status.SOURCE,
    request,
    blockNumber: blockNumber.toString(),
    blockHash: block.hash,
    transactionHash,
    blockTimestamp: block.timestamp,
  });
}
