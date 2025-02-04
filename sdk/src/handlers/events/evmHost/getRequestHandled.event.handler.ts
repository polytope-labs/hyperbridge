import { EventType } from "../../../types";
import { GetRequestHandledLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import { getHostStateMachine } from "../../../utils/substrate.helpers";

/**
 * Handles the GetRequestHandled event
 */
export async function handleGetRequestHandledEvent(
  event: GetRequestHandledLog
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
  const { commitment } = args;

  logger.info(
    `Handling GetRequestHandled Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string = getHostStateMachine(chainId);

  await EvmHostEventsService.createEvent(
    {
      commitment,
      transactionHash,
      transactionIndex,
      blockHash,
      blockNumber,
      data,
      timestamp: Number(block.timestamp),
      type: EventType.EVM_HOST_GET_REQUEST_HANDLED,
    },
    chain
  );
}
