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
  logger.info("Handling PostRequest event");
  const { transaction, blockNumber, transactionHash, args } = event;

  let { data, dest, fee, from, nonce, source, timeoutTimestamp, to } = args;

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
  await RequestService.findOrCreate(
    request_commitment,
    data,
    dest,
    BigInt(fee.toString()),
    from,
    BigInt(nonce.toString()),
    source,
    RequestStatus.SOURCE,
    BigInt(timeoutTimestamp.toString()),
    to,
  );

  // Create request meta data entity
  await RequestService.updateRequestStatus(
    request_commitment,
    RequestStatus.SOURCE,
    BigInt(blockNumber),
    transactionHash,
  );
}
