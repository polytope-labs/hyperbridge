import assert from "assert";
import { BidPlacedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { BidService } from "../../../services/bid.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the BidPlaced event
 */
export async function handleBidPlacedEvent(event: BidPlacedLog): Promise<void> {
  assert(event.args, "No handleBidPlacedEvent args");

  const { args, transaction, transactionHash, blockNumber } = event;
  const { commitment, assetId, bid, bidder } = args;

  logger.info(
    `Handling BidPlaced Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await BidService.createBid({
    id: commitment,
    commitment,
    assetId,
    bid: BigInt(bid.toString()),
    bidder,
    chain,
  });
}
