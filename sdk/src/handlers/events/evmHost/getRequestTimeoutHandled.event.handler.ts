import assert from "assert";
import { EventType } from "../../../types";
import { GetRequestTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the GetRequestTimeoutHandled event
 */
export async function handleGetRequestTimeoutHandledEvent(
  event: GetRequestTimeoutHandledLog
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
  const { commitment, dest } = args;

  logger.info(
    `Handling GetRequestTimeoutHandled Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

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
      type: EventType.EVM_HOST_GET_REQUEST_TIMEOUT_HANDLED,
    },
    chain
  );
}
