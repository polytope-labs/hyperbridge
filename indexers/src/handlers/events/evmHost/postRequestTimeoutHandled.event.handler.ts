import assert from "assert";
import { EventType, SupportedChain } from "../../../types";
import { PostRequestTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handles the PostRequestTimeoutHandled event
 */
export async function handlePostRequestTimeoutHandledEvent(
  event: PostRequestTimeoutHandledLog,
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
      type: EventType.EVM_HOST_POST_REQUEST_TIMEOUT_HANDLED,
    },
    chain,
  );
}
