// import { CHAINS_BY_ISMP_HOST } from '../../../constants';
// import { HyperBridgeService } from '../../../services/hyperbridge.service';
// import { RelayerService } from '../../../services/relayer.service';
// // import { HandlePostRequestsTransaction } from '../../../types/abi-interfaces/HandlerV1Abi';
// import { getHostStateMachine } from '../../../utils/substrate.helpers';

// /**
//  * Handles the handlePostRequest transaction from handlerV1 contract
//  */
// export async function handlePostRequestTransactionHandler(
//  transaction: any
// ): Promise<void> {
//  logger.info(
//   `Checking Incoming PostRequest Transaction Args: ${JSON.stringify(
//    transaction.args,
//   )}`
//  );

//  if (!transaction.args) {
//   logger.info('Not handling post request transaction - args is empty');
//   return;
//  }

//  const chain: string = getHostStateMachine(chainId);
//  logger.info(`Chain: ${chain}`);

//  const expectedHostAddress = CHAINS_BY_ISMP_HOST[chain];
//  const incomimgHostAddress = transaction.args![0];

//  if (incomimgHostAddress !== expectedHostAddress) {
//   logger.info(
//    `Skipping transaction - host address mismatch for chain ${chain}. Host address: ${incomimgHostAddress}, expected host address: ${expectedHostAddress}`
//   );
//   return;
//  }

//  const { blockNumber, hash } = transaction;

//  logger.info(
//   `Handling PostRequests Transaction: ${JSON.stringify({
//    blockNumber,
//    transactionHash: hash,
//   })}`
//  );

//  try {
//   await RelayerService.handlePostRequestOrResponseTransaction(
//    chain,
//    transaction
//   );

//   await HyperBridgeService.handlePostRequestOrResponseTransaction(
//    chain,
//    transaction
//   );
//  } catch (error) {
//   logger.error(
//    `Error while handling PostRequest transaction: ${JSON.stringify(error)}`
//   );
//  }
// }
