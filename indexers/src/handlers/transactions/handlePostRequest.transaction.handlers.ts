import assert from "assert";
import { HandlePostRequestsTransaction } from "../../types/abi-interfaces/HandlerV1Abi";
import { RelayerService } from "../../services/relayer.service";
import {
  getSupportedChainByChainId,
  hexToDecimal,
} from "../../utils/chain.helpers";

/**
 * Handles the handlePostRequest transaction from Hyperbridge
 */
export async function handlePostRequestTransaction(
  transaction: HandlePostRequestsTransaction,
): Promise<void> {
  assert(
    transaction.args,
    "No handlePostRequestTransaction args found in ${network} network",
  );

  const chainId = transaction.chainId ? hexToDecimal(transaction.chainId) : "";
  const chain = getSupportedChainByChainId(chainId);

  if (!chain) {
    throw new Error(
      `Unsupported chainId ${chainId} for handlePostRequest transaction`,
    );
  }

  await RelayerService.handlePostRequestTransaction(transaction, chain);
}
