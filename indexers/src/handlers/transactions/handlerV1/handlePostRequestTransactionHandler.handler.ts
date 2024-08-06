import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { HandlePostRequestsTransaction } from "../../../types/abi-interfaces/HandlerV1Abi";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the handlePostRequest transaction from handlerV1 contract
 */
export async function handlePostRequestTransactionHandler(
  transaction: HandlePostRequestsTransaction
): Promise<void> {
  assert(transaction.args);
  const { blockNumber, hash } = transaction;

  logger.info(
    `Handling PostRequests Transaction: ${JSON.stringify({
      blockNumber,
      transactionHash: hash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  Promise.all([
    await RelayerService.handlePostRequestOrResponseTransaction(
      chain,
      transaction
    ),
    await HyperBridgeService.handlePostRequestOrResponseTransaction(
      chain,
      transaction
    ),
  ]);
}
