import assert from "assert";
import { EventType, SupportedChain } from "../../../types";
import { GetRequestHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handles the GetRequestHandled event
 */
export async function handleGetRequestHandledEvent(
  event: GetRequestHandledLog,
): Promise<void> {
  assert(event.args, "No handleGetRequestHandledEvent args");

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
      type: EventType.EVM_HOST_GET_REQUEST_HANDLED,
    },
    chain,
  );
}
