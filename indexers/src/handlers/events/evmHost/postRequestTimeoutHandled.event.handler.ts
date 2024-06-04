import assert from "assert";
import { EventType, Status, SupportedChain } from "../../../types";
import { PostRequestTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RequestService } from "../../../services/request.service";

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
    data,
  } = event;
  const { commitment, dest } = args;

  logger.info(
    `Handling PostRequestTimeoutHandled Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

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
        dest,
        timestamp: Number(block.timestamp),
        type: EventType.EVM_HOST_POST_REQUEST_TIMEOUT_HANDLED,
      },
      chain,
    ),
    await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain),
    await RequestService.updateStatus({
      commitment,
      chain,
      blockNumber: blockNumber.toString(),
      blockHash: block.hash,
      blockTimestamp: block.timestamp,
      status: Status.TIMED_OUT,
      transactionHash,
    }),
  ]);
}
