import assert from "assert";
import { EventType } from "../../../types";
import { GetRequestHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the GetRequestHandled event
 */
export async function handleGetRequestHandledEvent(
  event: GetRequestHandledLog
): Promise<void> {
  if(!event.args) return;

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
  const { commitment } = args;

  logger.info(
    `Handling GetRequestHandled Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await EvmHostEventsService.createEvent(
    {
      commitment,
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      data,
      timestamp: Number(block.timestamp),
      type: EventType.EVM_HOST_GET_REQUEST_HANDLED,
    },
    chain
  );
}
