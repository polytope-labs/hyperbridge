import assert from "assert";
import { SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { PostResponseEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";

/**
 * Handles the PostResponse event from Evm Hosts
 */
export async function handlePostResponseEvent(
  event: PostResponseEventLog,
): Promise<void> {
  assert(event.args, "No handlePostResponseEvent args");
  logger.info("Handling PostResponse event");

  const { transaction } = event;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event);
}
