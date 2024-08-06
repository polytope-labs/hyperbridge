import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RequestService } from "../../../services/request.service";
import { Status } from "../../../types";
import { PostRequestEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the PostRequest event from Evm Hosts
 */
export async function handlePostRequestEvent(
  event: PostRequestEventLog
): Promise<void> {
  assert(event.args, "No handlePostRequestEvent args");

  const { transaction, blockNumber, transactionHash, args, block } = event;
  let { dest, fee, from, nonce, source, timeoutTimestamp, to, body } = args;

  logger.info(
    `Handling PostRequest Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event);

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

  // Create the request entity
  await RequestService.findOrCreate({
    chain,
    commitment: request_commitment,
    body,
    dest,
    fee: BigInt(fee.toString()),
    from,
    nonce: BigInt(nonce.toString()),
    source,
    status: Status.SOURCE,
    timeoutTimestamp: BigInt(timeoutTimestamp.toString()),
    to,
    blockNumber: blockNumber.toString(),
    blockHash: block.hash,
    transactionHash,
    blockTimestamp: block.timestamp,
  });
}
