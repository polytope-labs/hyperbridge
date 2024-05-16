import assert from "assert";
import { RequestStatus, SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { PostRequestEventLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RequestService } from "../../../services/request.service";

/**
 * Handles the PostRequest event from Evm Hosts
 */
export async function handlePostRequestEvent(
  event: PostRequestEventLog,
): Promise<void> {
  assert(event.args, "No handlePostRequestEvent args");

  const { transaction, blockNumber, transactionHash, args, block } = event;
  let { data, dest, fee, from, nonce, source, timeoutTimestamp, to } = args;

  logger.info(
    `Handling PostRequest Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event);

  // Compute the request commitment
  let request_commitment = RequestService.computeRequestCommitment(
    source,
    dest,
    BigInt(nonce.toString()),
    BigInt(timeoutTimestamp.toString()),
    from,
    to,
    data,
  );

  // Create the request entity
  await RequestService.findOrCreate({
    chain,
    commitment: request_commitment,
    data,
    dest,
    fee: BigInt(fee.toString()),
    from,
    nonce: BigInt(nonce.toString()),
    source,
    status: RequestStatus.SOURCE,
    timeoutTimestamp: BigInt(timeoutTimestamp.toString()),
    to,
    blockNumber: blockNumber.toString(),
    blockHash: block.hash,
    transactionHash,
    blockTimestamp: block.timestamp,
  });
}
