import assert from 'assert';

import { HyperBridgeService } from '../../../services/hyperbridge.service';
import { EventType, Status } from '../../../types';
import { PostResponseHandledLog } from '../../../types/abi-interfaces/EthereumHostAbi';
import { EvmHostEventsService } from '../../../services/evmHostEvents.service';
import { ResponseService } from '../../../services/response.service';
import StateMachineHelpers from '../../../utils/stateMachine.helpers';

/**
 * Handles the PostResponseHandled event from Hyperbridge
 */
export async function handlePostResponseHandledEvent(
 event: PostResponseHandledLog
): Promise<void> {
 if (!event.args) return;

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

 logger.info(
  `Handling PostResponseHandled Event: ${JSON.stringify({
   blockNumber,
   transactionHash,
  })}`
 );

 const chain: string =
  StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

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
   chain
  ),
  await HyperBridgeService.handlePostRequestOrResponseHandledEvent(
   relayer_id,
   chain
  ),

  await ResponseService.updateStatus({
   commitment,
   chain,
   blockNumber: blockNumber.toString(),
   blockTimestamp: block.timestamp,
   blockHash: block.hash,
   status: Status.DESTINATION,
   transactionHash,
  }),
 ]);
}
