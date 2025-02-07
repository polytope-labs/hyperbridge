import {  Status } from '../../../types';
import { PostResponseTimeoutHandledLog } from '../../../types/abi-interfaces/EthereumHostAbi';
import { HyperBridgeService } from '../../../services/hyperbridge.service';
import { ResponseService } from '../../../services/response.service';
import { getHostStateMachine } from '../../../utils/substrate.helpers';

/**
 * Handles the PostResponseTimeoutHandled event
 */
export async function handlePostResponseTimeoutHandledEvent(
 event: PostResponseTimeoutHandledLog
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
 const { commitment, dest } = args;

 logger.info(
  `Handling PostResponseTimeoutHandled Event: ${JSON.stringify({
   blockNumber,
   transactionHash,
  })}`
 );

 const chain: string = getHostStateMachine(chainId);

 try {
  await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain);

  await ResponseService.updateStatus({
   commitment,
   chain,
   blockNumber: blockNumber.toString(),
   blockHash: block.hash,
   blockTimestamp: block.timestamp,
   status: Status.TIMED_OUT,
   transactionHash,
  });
 } catch (error) {
  logger.error(
   `Error updating handling post response timeout: ${JSON.stringify(error)}`
  );
 }
}
