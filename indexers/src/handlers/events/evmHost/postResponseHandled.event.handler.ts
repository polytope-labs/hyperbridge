import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { EventType, SupportedChain } from "../../../types";
import { PostResponseHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handles the PostResponseHandled event from Hyperbridge
 */
export async function handlePostResponseHandledEvent(
  event: PostResponseHandledLog,
): Promise<void> {
  assert(event.args, "No handlePostResponseHandledEvent args");

  const {
    args,
    block,
    transaction,
    transactionHash,
    transactionIndex,
    blockHash,
    blockNumber,
  } = event;
  const { relayer, commitment } = args;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  Promise.all([
    await EvmHostEventsService.createEvent(
      {
        commitment,
        transactionHash,
        transactionIndex,
        blockHash,
        blockNumber,
        timestamp: Number(block.timestamp),
        type: EventType.EVM_HOST_POST_RESPONSE_HANDLED,
      },
      chain,
    ),
    await RelayerService.incrementNumberOfPostRequestsHandled(relayer, chain),
    await HyperBridgeService.incrementNumberOfPostRequestsHandled(chain),
  ]);
}
