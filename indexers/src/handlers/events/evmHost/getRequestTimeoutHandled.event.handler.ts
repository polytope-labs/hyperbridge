import assert from "assert";
import { EventType, SupportedChain } from "../../../types";
import { GetRequestTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handles the GetRequestTimeoutHandled event
 */
export async function handleGetRequestTimeoutHandledEvent(
  event: GetRequestTimeoutHandledLog,
): Promise<void> {
  assert(event.args, "No handleGetRequestTimeoutHandledEvent args");

  const {
    args,
    block,
    transaction,
    transactionHash,
    transactionIndex,
    blockHash,
    blockNumber,
  } = event;
  const { commitment } = args;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await EvmHostEventsService.createEvent(
    {
      commitment,
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      timestamp: Number(block.timestamp),
      type: EventType.EVM_HOST_GET_REQUEST_TIMEOUT_HANDLED,
    },
    chain,
  );
}
