import { ProtocolParticipant, RewardPoints, SupportedChain } from "../types";
import { TokenGatewayService } from "./tokenGateway.service";

export interface IAssignRewardPointsForFulfilledRequestInput {
  address: string;
  chain: SupportedChain;
  amount: bigint;
  earnerType: ProtocolParticipant;
  asset_id: string;
  contract_address: string;
}

export interface IAssignRewardPointsToRelayerInput {
  relayer_address: string;
  chain: SupportedChain;
  is_success: boolean;
  earnerType: ProtocolParticipant;
}

export interface IAssignRewardPointsForAssetTransferInput {
  address: string;
  chain: SupportedChain;
  amount: bigint;
  earnerType: ProtocolParticipant;
  asset_id: string;
  contract_address: string;
}

const REWARD_POINTS_TO_RELAYER_ON_SUCCESSFUL_TRANSACTION = BigInt(10);
const REWARD_POINTS_TO_RELAYER_ON_FAILED_TRANSACTION = BigInt(1);

export class RewardPointsService {
  static async assignRewardForFulfilledRequest(
    data: IAssignRewardPointsForFulfilledRequestInput,
  ) {
    const { address, chain, amount, earnerType, asset_id, contract_address } =
      data;

    const usdValue = await TokenGatewayService.getUsdValueOfAsset(
      chain,
      contract_address,
      asset_id,
      amount,
    );

    let rewardPointRecord = await RewardPoints.get(
      `${address}-${chain}-${earnerType}`,
    );

    if (typeof rewardPointRecord === "undefined") {
      rewardPointRecord = RewardPoints.create({
        id: `${address}-${chain}-${earnerType}`,
        address,
        chain,
        earnerType,
        points: usdValue,
      });
    } else {
      rewardPointRecord.points += usdValue;
    }

    await rewardPointRecord.save();
  }

  static async assignRewardToRelayer(data: IAssignRewardPointsToRelayerInput) {
    const { chain, relayer_address, is_success, earnerType } = data;

    let rewardPointRecord = await RewardPoints.get(
      `${relayer_address}-${chain}-${earnerType}`,
    );

    const points = is_success
      ? REWARD_POINTS_TO_RELAYER_ON_SUCCESSFUL_TRANSACTION
      : REWARD_POINTS_TO_RELAYER_ON_FAILED_TRANSACTION;

    if (typeof rewardPointRecord === "undefined") {
      rewardPointRecord = RewardPoints.create({
        id: `${relayer_address}-${chain}-${earnerType}`,
        address: relayer_address,
        chain,
        earnerType,
        points,
      });
    } else {
      rewardPointRecord.points += points;
    }

    await rewardPointRecord.save();
  }

  static async assignRewardForAssetTransfer(
    data: IAssignRewardPointsForAssetTransferInput,
  ) {
    const { address, chain, amount, earnerType, asset_id, contract_address } =
      data;

    const usdValue = await TokenGatewayService.getUsdValueOfAsset(
      chain,
      contract_address,
      asset_id,
      amount,
    );

    let rewardPointRecord = await RewardPoints.get(
      `${address}-${chain}-${earnerType}`,
    );

    if (typeof rewardPointRecord === "undefined") {
      rewardPointRecord = RewardPoints.create({
        id: `${address}-${chain}-${earnerType}`,
        address,
        chain,
        earnerType,
        points: usdValue,
      });
    } else {
      rewardPointRecord.points += usdValue;
    }

    await rewardPointRecord.save();
  }
}
