import assert from "assert";
import { EventType, Status } from "../../../types";
import { PostRequestTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RequestService } from "../../../services/request.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the PostRequestTimeoutHandled event
 */
export async function handlePostRequestTimeoutHandledEvent(
  event: PostRequestTimeoutHandledLog
): Promise<void> {
  assert(event.args, "No handlePostRequestTimeoutHandledEvent args");

  const {
    args,
    block,
    transaction,
    transactionHash,
    transactionIndex,
    blockHash,
    blockNumber,
    data,
  } = event;
  const { commitment, dest } = args;

  logger.info(
    `Handling PostRequestTimeoutHandled Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  Promise.all([
    await EvmHostEventsService.createEvent(
      {
        data,
        commitment,
        transactionHash,
        transactionIndex,
        blockHash,
        blockNumber,
        dest,
        timestamp: Number(block.timestamp),
        type: EventType.EVM_HOST_POST_REQUEST_TIMEOUT_HANDLED,
      },
      chain
    ),
    await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain),
    await RequestService.updateStatus({
      commitment,
      chain,
      blockNumber: blockNumber.toString(),
      blockHash: block.hash,
      blockTimestamp: block.timestamp,
      status: Status.TIMED_OUT,
      transactionHash,
    }),
  ]);
}
