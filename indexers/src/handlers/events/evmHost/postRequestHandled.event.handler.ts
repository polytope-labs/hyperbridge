import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { EventType, SupportedChain } from "../../../types";
import { PostRequestHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
export async function handlePostRequestHandledEvent(
  event: PostRequestHandledLog,
): Promise<void> {
  assert(event.args, "No handlePostRequestHandledEvent args");
  logger.info("Handling PostRequestHandled event " + event.blockNumber);

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
  const { relayer: relayer_id, commitment } = args;
  const { status } = await transaction.receipt();

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  Promise.all([
    await EvmHostEventsService.createEvent(
      {
        data,
        commitment,
        transactionHash,
        transactionIndex,
        blockHash,
        blockNumber,
        timestamp: Number(block.timestamp),
        type: EventType.EVM_HOST_POST_REQUEST_HANDLED,
      },
      chain,
    ),
    await HyperBridgeService.handlePostRequestOrResponseHandledEvent(
      relayer_id,
      chain,
      status,
    ),
    await RelayerService.handlePostRequestOrPostResponseHandledEvent(
      relayer_id,
      transaction,
      chain,
    ),
  ]);
}
