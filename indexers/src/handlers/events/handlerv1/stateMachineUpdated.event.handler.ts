import assert from "assert";
import { StateMachineUpdatedLog } from "../../../types/abi-interfaces/HandlerV1Abi";
import { EventType, SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { HandlerV1EventsService } from "../../../services/handlerV1Events.service";

/**
 * Handle the StateMachineUpdated event on the HandlerV1 contract
 */
export async function handleStateMachineUpdatedEvent(
  event: StateMachineUpdatedLog,
): Promise<void> {
  assert(event.args, "No handleStateMachineUpdatedEvent args");

  const {
    blockHash,
    blockNumber,
    transactionHash,
    transactionIndex,
    block,
    transaction,
    args,
  } = event;

  const { stateMachineId, height } = args;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);
  await HandlerV1EventsService.createEvent(
    {
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      timestamp: Number(block.timestamp),
      type: EventType.HANDLER_V1_STATE_MACHINE_UPDATED,
      metadata: {
        stateMachineId: stateMachineId.toBigInt(),
        height: height.toBigInt(),
      },
    },
    chain,
  );
}
