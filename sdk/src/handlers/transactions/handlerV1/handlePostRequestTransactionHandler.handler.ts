import { HyperBridgeService } from '../../../services/hyperbridge.service';
import { RelayerService } from '../../../services/relayer.service';
import { HandlePostRequestsTransaction } from '../../../types/abi-interfaces/HandlerV1Abi';
import { getHostStateMachine } from '../../../utils/substrate.helpers';

/**
 * Handles the handlePostRequest transaction from handlerV1 contract
 */
export async function handlePostRequestTransactionHandler(
 transaction: HandlePostRequestsTransaction
): Promise<void> {
 if (!!transaction.args) return;

 const { blockNumber, hash } = transaction;

 logger.info(
  `Handling PostRequests Transaction: ${JSON.stringify({
   blockNumber,
   transactionHash: hash,
  })}`
 );

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
 } catch (error) {
  logger.error(
   `Error while handling PostRequest transaction: ${JSON.stringify(error)}`
  );
 }
}
