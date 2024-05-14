import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { HandlePostRequestsTransaction } from "../../../types/abi-interfaces/HandlerV1Abi";
import { SupportedChain } from "../../../types/enums";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";

/**
 * Handles the handlePostRequest transaction from handlerV1 contract
 */
export async function handlePostRequestTransactionHandler(
  transaction: HandlePostRequestsTransaction,
): Promise<void> {
  logger.info(
    `New handlePostRequest transaction at block ${transaction.blockNumber}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  Promise.all([
    await RelayerService.handlePostRequestOrResponseTransaction(
      chain,
      transaction,
    ),
    await HyperBridgeService.handlePostRequestOrResponseTransaction(
      chain,
      transaction,
    ),
  ]);
}
