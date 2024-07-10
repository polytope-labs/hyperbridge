import assert from "assert";
import { SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { BidPlacedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { BidService } from "../../../services/bid.service";

/**
 * Handles the BidPlaced event
 */
export async function handleBidPlacedEvent(event: BidPlacedLog): Promise<void> {
  assert(event.args, "No handleBidPlacedEvent args");

  const { args, transaction, transactionHash, blockNumber } = event;
  const { commitment, assetId, bid, bidder } = args;

  logger.info(
    `Handling BidPlaced Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await BidService.createBid({
    id: commitment,
    commitment,
    assetId,
    bid: BigInt(bid.toString()),
    bidder,
    chain,
  });
}
