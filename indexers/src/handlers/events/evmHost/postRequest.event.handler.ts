import assert from "assert";
import { SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { PostRequestEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";

/**
 * Handles the PostRequest event from Evm Hosts
 */
export async function handlePostRequestEvent(
  event: PostRequestEventLog,
): Promise<void> {
  logger.info("Handling PostRequest event");
  logger.debuf("Handling PostRequest event");
  assert(event.args, "No handlePostRequestEvent args");
  const {
    blockHash,
    blockNumber,
    transactionHash,
    transactionIndex,
    block,
    transaction,
    args,
  } = event;
  const { fee, data } = args;
  const chain: SupportedChain = getEvmChainFromTransaction(transaction);
  logger.info("Handling PostRequest event");
  logger.info(JSON.stringify(data));
  // @Todo Add logic handling PostRequestEvents
}
