import assert from "assert";
import { ProtocolParticipant } from "../../../types";
import { AssetTeleportedLog } from "../../../types/abi-interfaces/TokenGatewayAbi";
import { AssetService } from "../../../services/asset.service";
import { RewardPointsService } from "../../../services/reward-points.service";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";

/**
 * Handles the AssetTeleported event
 */
export async function handleAssetTeleportedEvent(
  event: AssetTeleportedLog
): Promise<void> {
  assert(event.args, "No handleAssetTeleportedEvent args");

  const {
    args,
    transaction,
    transactionHash,
    blockNumber,
    address: contract_address,
  } = event;
  const { commitment, from, to, amount, assetId, redeem } = args;

  logger.info(
    `Handling AssetTeleported Event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  await AssetService.createTeleportedAsset({
    id: commitment,
    commitment,
    amount: BigInt(amount.toString()),
    assetId,
    to,
    from,
    chain,
    redeem,
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
