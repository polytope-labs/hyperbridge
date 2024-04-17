import assert from "assert";
import { SupportedChain } from "../../types";
import { HandlePostRequestsTransaction } from "../../types/abi-interfaces/HandlerV1Abi";
import { RelayerService } from "../../services/relayer.service";

/**
 * Handles the handlePostRequest transaction from Hyperbridge
 */
async function handlePostRequestTransaction(
  transaction: HandlePostRequestsTransaction,
  chain: SupportedChain,
): Promise<void> {
  assert(
    transaction.args,
    "No handlePostRequestTransaction args found in ${network} network",
  );

  await RelayerService.handlePostRequestTransaction(transaction, chain);
}

// Handle the handlePostRequest transaction for the Ethereum Sepolia chain
export async function handleEthereumSepoliaPostRequestTransactionHandler(
  transaction: HandlePostRequestsTransaction,
): Promise<void> {
  await handlePostRequestTransaction(
    transaction,
    SupportedChain.ETHEREUM_SEPOLIA,
  );
}
