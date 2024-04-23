import assert from "assert";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { EventType, SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { PostResponseEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";

/**
 * Handles the PostResponse event from Evm Hosts
 */
export async function handlePostResponseEvent(
  event: PostResponseEventLog,
): Promise<void> {
  assert(event.args, "No handlePostResponseEvent args");

  const {
    blockHash,
    blockNumber,
    transactionHash,
    transactionIndex,
    block,
    transaction,
  } = event;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await EvmHostEventsService.createEvent(
    {
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      timestamp: Number(block.timestamp),
      type: EventType.EVM_HOST_POST_RESPONSE,
    },
    chain,
  );
}
