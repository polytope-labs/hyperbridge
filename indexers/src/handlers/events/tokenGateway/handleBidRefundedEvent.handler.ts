import assert from "assert";
import { BidRefundedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { BidService } from "../../../services/bid.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the BidRefunded event
 */
export async function handleBidRefundedEvent(
  event: BidRefundedLog
): Promise<void> {
  assert(event.args, "No handleBidRefundedEvent args");

  const { args, transaction, transactionHash, blockNumber } = event;
  const { commitment, assetId, bidder } = args;

  logger.info(
    `Handling BidRefunded Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await BidService.createBidRefund({
    id: commitment,
    commitment,
    assetId,
    bidder,
    chain,
  });
}
