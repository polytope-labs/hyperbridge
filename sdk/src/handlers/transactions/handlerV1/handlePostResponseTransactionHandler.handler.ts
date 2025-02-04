import { HyperBridgeService } from '../../../services/hyperbridge.service';
import { RelayerService } from '../../../services/relayer.service';
import { HandlePostRequestsTransaction } from '../../../types/abi-interfaces/HandlerV1Abi';
import { getHostStateMachine } from '../../../utils/substrate.helpers';

/**
 * Handles the handlePostResponse transaction from handlerV1 contract
 */
export async function handlePostResponseTransactionHandler(
 transaction: HandlePostRequestsTransaction
): Promise<void> {
 const { blockNumber, hash } = transaction;

 logger.info(
  `Handling PostRequests Transaction: ${JSON.stringify({
   blockNumber,
   transactionHash: hash,
  })}`
 );

 const { status } = await transaction.receipt();

 // Check if transaction has revert data
 if (!status) {
  logger.info(`Transaction ${hash} was reverted, skipping processing`);
  return;
 }

 const chain: string = getHostStateMachine(chainId);
 logger.info(`Chain: ${chain}`);

 try {
  // await RelayerService.handlePostRequestOrResponseTransaction(
  //  chain,
  //  transaction
  // );
  await HyperBridgeService.handlePostRequestOrResponseTransaction(
   chain,
   transaction
  );
 } catch (error: unknown) {
  logger.error(
   `Error handling PostRequests Transaction: ${JSON.stringify({
    blockNumber,
    transactionHash: hash,
    error,
   })}`
  );
 }
}
