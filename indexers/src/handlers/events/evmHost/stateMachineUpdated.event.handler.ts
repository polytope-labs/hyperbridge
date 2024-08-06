import assert from "assert";
import { StateMachineUpdatedLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { StateMachineService } from "../../../services/stateMachine.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handle the StateMachineUpdated event
 */
export async function handleStateMachineUpdatedEvent(
  event: StateMachineUpdatedLog
): Promise<void> {
  assert(
    event.args,
    `No handleStateMachineUpdatedEvent args. Tx Hash: ${event.transactionHash}`
  );
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

  logger.info(
    `Handling StateMachineUpdated Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);
  await StateMachineService.createEvmStateMachineUpdatedEvent(
    {
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      timestamp: Number(block.timestamp),
      stateMachineId: stateMachineId,
      height: height.toNumber(),
    },
    chain
  );
}
