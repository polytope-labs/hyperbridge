import assert from "assert";
import { SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { BidRefundedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { BidService } from "../../../services/bid.service";

/**
 * Handles the BidRefunded event
 */
export async function handleBidRefundedEvent(
  event: BidRefundedLog,
): Promise<void> {
  assert(event.args, "No handleBidRefundedEvent args");

  const { args, transaction, transactionHash, blockNumber } = event;
  const { commitment, assetId, bidder } = args;

  logger.info(
    `Handling BidRefunded Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await BidService.createBidRefund({
    id: commitment,
    commitment,
    assetId,
    bidder,
    chain,
  });
}
