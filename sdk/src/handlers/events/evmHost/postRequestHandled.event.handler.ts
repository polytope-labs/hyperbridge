import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { EventType, Status } from "../../../types";
import { PostRequestHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { RequestService } from "../../../services/request.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
export async function handlePostRequestHandledEvent(
  event: PostRequestHandledLog
): Promise<void> {
  assert(event.args, "No handlePostRequestHandledEvent args");

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
  const { relayer: relayer_id, commitment } = args;

  logger.info(
    `Handling PostRequestHandled Event: ${JSON.stringify({
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
        timestamp: Number(block.timestamp),
        type: EventType.EVM_HOST_POST_REQUEST_HANDLED,
      },
      chain
    ),
    await HyperBridgeService.handlePostRequestOrResponseHandledEvent(
      relayer_id,
      chain
    ),
    await RequestService.updateStatus({
      commitment,
      chain,
      blockNumber: blockNumber.toString(),
      blockHash: block.hash,
      blockTimestamp: block.timestamp,
      status: Status.DEST,
      transactionHash,
    }),
  ]);
}
