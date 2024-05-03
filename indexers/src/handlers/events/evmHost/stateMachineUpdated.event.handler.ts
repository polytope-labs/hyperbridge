import assert from "assert";
import { SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { StateMachineUpdatedLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handle the StateMachineUpdated event
 */
export async function handleStateMachineUpdatedEvent(
  event: StateMachineUpdatedLog,
): Promise<void> {
  assert(
    event.args,
    `No handleStateMachineUpdatedEvent args. Tx Hash: ${event.transactionHash}`,
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
  const chain: SupportedChain = getEvmChainFromTransaction(transaction);
  await EvmHostEventsService.createStateMachineUpdatedEvent(
    {
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      timestamp: Number(block.timestamp),
      stateMachineId: stateMachineId,
      height: height.toBigInt(),
    },
    chain,
  );
}
