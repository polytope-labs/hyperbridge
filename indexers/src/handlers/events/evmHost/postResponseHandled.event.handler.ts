import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { EventType, RequestStatus, SupportedChain } from "../../../types";
import { PostResponseHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { RequestService } from "../../../services/request.service";

/**
 * Handles the PostResponseHandled event from Hyperbridge
 */
export async function handlePostResponseHandledEvent(
  event: PostResponseHandledLog,
): Promise<void> {
  assert(event.args, "No handlePostResponseHandledEvent args");
  logger.info("Handling PostResponseHandled event: " + event.blockNumber);

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
        type: EventType.EVM_HOST_POST_RESPONSE_HANDLED,
      },
      chain,
    ),
    await HyperBridgeService.handlePostRequestOrResponseHandledEvent(
      relayer_id,
      chain,
    ),
    await RelayerService.handlePostRequestOrPostResponseHandledEvent(
      relayer_id,
      transaction,
      chain,
    ),
    await RequestService.updateRequestStatus(
      commitment,
      RequestStatus.DEST,
      BigInt(blockNumber),
      transactionHash,
    ),
  ]);
}
