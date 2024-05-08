import { RelayerService } from "../../../services/relayer.service";
import { HandlePostRequestsTransaction } from "../../../types/abi-interfaces/HandlerV1Abi";
import { SupportedChain } from "../../../types/enums";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";

/**
 * Handles the handlePostResponse transaction from handlerV1 contract
 */
export async function handlePostResponseTransactionHandler(
  transaction: HandlePostRequestsTransaction,
): Promise<void> {
  logger.info(
    `New handlePostResponse trnasaction at block ${transaction.blockNumber}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await RelayerService.handlePostRequestOrResponseTransaction(
    chain,
    transaction,
  );
}
