import {
  ProtocolParticipant,
  RewardPoints,
  RewardPointsActivityLog,
  RewardPointsActivityType,
} from "../types";
import { TokenGatewayService } from "./tokenGateway.service";

export interface IAssignRewardPointsForFulfilledRequestInput {
  address: string;
  chain: string;
  amount: bigint;
  earnerType: ProtocolParticipant;
  asset_id: string;
  contract_address: string;
  transaction_hash: string;
}

export interface IAssignRewardPointsToRelayerInput {
  relayer_address: string;
  chain: string;
  is_success: boolean;
  earnerType: ProtocolParticipant;
  transaction_hash: string;
}

export interface IAssignRewardPointsForAssetTransferInput {
  address: string;
  chain: string;
  amount: bigint;
  earnerType: ProtocolParticipant;
  asset_id: string;
  contract_address: string;
  transaction_hash: string;
}

const REWARD_POINTS_TO_RELAYER_ON_SUCCESSFUL_TRANSACTION = BigInt(10);
const REWARD_POINTS_TO_RELAYER_ON_FAILED_TRANSACTION = BigInt(1);

export class RewardPointsService {
  /**
   * Assign a reward for fulfilled requests
   * @param data
   */
  static async assignRewardForFulfilledRequest(
    data: IAssignRewardPointsForFulfilledRequestInput
  ) {
    const {
      address,
      chain,
      amount,
      earnerType,
      asset_id,
      contract_address,
      transaction_hash,
    } = data;

    const usdValue = await TokenGatewayService.getUsdValueOfAsset(
      chain,
      contract_address,
      asset_id,
      amount
    );

    let rewardPointRecord = await RewardPoints.get(
      `${address}-${chain}-${earnerType}`
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
    await RewardPointsActivityLog.create({
      id: `${address}-${earnerType}-${transaction_hash}`,
      chain,
      points: usdValue,
      transactionHash: transaction_hash,
      earnerAddress: address,
      earnerType,
      activityType: RewardPointsActivityType.REWARD_POINTS_EARNED,
      description: "Reward points awarded for fulfilling a request",
      createdAt: new Date(),
    }).save();
  }

  /**
   * Assign reward points to a relayer based on the success/failure of a transaction
   * @param data
   */
  static async assignRewardToRelayer(data: IAssignRewardPointsToRelayerInput) {
    const { chain, relayer_address, is_success, earnerType, transaction_hash } =
      data;

    let rewardPointRecord = await RewardPoints.get(
      `${relayer_address}-${chain}-${earnerType}`
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

    const description = is_success
      ? "Reward points earned for successfully relaying a message"
      : "Reward points earned for failed relaying of a message";
    await RewardPointsActivityLog.create({
      id: `${relayer_address}-${earnerType}-${transaction_hash}`,
      chain,
      points,
      transactionHash: transaction_hash,
      earnerAddress: relayer_address,
      earnerType,
      activityType: RewardPointsActivityType.REWARD_POINTS_EARNED,
      description,
      createdAt: new Date(),
    }).save();
  }

  /**
   * Assign rewards for asset transfer
   * @param data
   */
  static async assignRewardForAssetTransfer(
    data: IAssignRewardPointsForAssetTransferInput
  ) {
    const {
      address,
      chain,
      amount,
      earnerType,
      asset_id,
      contract_address,
      transaction_hash,
    } = data;

    const usdValue = await TokenGatewayService.getUsdValueOfAsset(
      chain,
      contract_address,
      asset_id,
      amount
    );

    let rewardPointRecord = await RewardPoints.get(
      `${address}-${chain}-${earnerType}`
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

    await RewardPointsActivityLog.create({
      id: `${address}-${earnerType}-${transaction_hash}`,
      chain,
      points: usdValue,
      transactionHash: transaction_hash,
      earnerAddress: address,
      earnerType,
      activityType: RewardPointsActivityType.REWARD_POINTS_EARNED,
      description: "Rewards points earned for asset transfer",
      createdAt: new Date(),
    }).save();
  }
}
