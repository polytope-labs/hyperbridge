import assert from "assert";
import { ProtocolParticipant, SupportedChain } from "../../../types";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";
import { RequestFulfilledLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { BidService } from "../../../services/bid.service";
import { RewardPointsService } from "../../../services/reward-points.service";
import { TokenGatewayService } from "../../../services/tokenGateway.service";

/**
 * Handles the RequestFulfilled event
 */
export async function handleRequestFulfilledEvent(
  event: RequestFulfilledLog,
): Promise<void> {
  assert(event.args, "No handleRequestFulfilledEvent args");

  const {
    args,
    transaction,
    transactionHash,
    blockNumber,
    address: contract_address,
  } = event;
  const { assetId, amount, bidder } = args;

  logger.info(
    `Handling RequestFulfilled Event: ${JSON.stringify({ blockNumber, transactionHash })}`,
  );

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

  await BidService.createFulfilledRequest({
    id: transactionHash,
    amount: BigInt(amount.toString()),
    assetId,
    bidder,
    chain,
  });

  const assetDetails = await TokenGatewayService.getAssetDetails(
    contract_address,
    assetId,
  );

  // If asset is an ERC20 token, assign reward points to the filler
  if (assetDetails.is_erc20) {
    await RewardPointsService.assignRewardForFulfilledRequest({
      address: bidder,
      chain,
      earnerType: ProtocolParticipant.FILLER,
      amount: BigInt(amount.toString()),
      asset_id: assetId,
      contract_address,
      transaction_hash: transactionHash,
    });
  }
}
