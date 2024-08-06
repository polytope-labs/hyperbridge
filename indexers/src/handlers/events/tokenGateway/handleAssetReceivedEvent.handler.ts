import assert from "assert";
import { ProtocolParticipant } from "../../../types";
import { AssetReceivedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { AssetService } from "../../../services/asset.service";
import { RewardPointsService } from "../../../services/reward-points.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the AssetReceived event
 */
export async function handleAssetReceivedEvent(
  event: AssetReceivedLog
): Promise<void> {
  assert(event.args, "No handleAssetReceivedEvent args");

  const {
    args,
    transaction,
    transactionHash,
    blockNumber,
    address: contract_address,
  } = event;
  const { commitment, from, beneficiary, amount, assetId } = args;

  logger.info(
    `Handling AssetReceived Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await AssetService.createReceivedAsset({
    id: commitment,
    commitment,
    amount: BigInt(amount.toString()),
    assetId,
    beneficiary,
    from,
    chain,
  });

  await RewardPointsService.assignRewardForAssetTransfer({
    address: from,
    chain,
    earnerType: ProtocolParticipant.USER,
    amount: BigInt(amount.toString()),
    asset_id: assetId,
    contract_address,
    transaction_hash: transactionHash,
  });
}
