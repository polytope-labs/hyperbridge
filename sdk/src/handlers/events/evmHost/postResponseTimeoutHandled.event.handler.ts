import { EventType, Status } from "../../../types";
import { PostResponseTimeoutHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { ResponseService } from "../../../services/response.service";
import { getHostStateMachine } from "../../../utils/substrate.helpers";

/**
 * Handles the PostResponseTimeoutHandled event
 */
export async function handlePostResponseTimeoutHandledEvent(
  event: PostResponseTimeoutHandledLog
): Promise<void> {
 if(!event.args) return;
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
    `Handling PostResponseTimeoutHandled Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string = getHostStateMachine(chainId);

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
        type: EventType.EVM_HOST_POST_RESPONSE_TIMEOUT_HANDLED,
      },
      chain
    ),
    await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain),
    await ResponseService.updateStatus({
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
