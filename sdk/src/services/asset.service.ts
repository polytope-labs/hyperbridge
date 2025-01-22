import {
 AssetReceived,
 AssetReceivedProps,
} from '../types/models/AssetReceived';
import {
 AssetTeleported,
 AssetTeleportedProps,
} from '../types/models/AssetTeleported';

export class AssetService {
 /**
  * Create a new received asset entity
  */
 static async createReceivedAsset(data: AssetReceivedProps) {
  let assetReceived = await AssetReceived.get(data.id);

  if (typeof assetReceived === 'undefined') {
   assetReceived = AssetReceived.create(data);
   await assetReceived.save();
  } else {
   logger.info(
    `Attempted to create new asset received entity with existing id ${data.id}`
   );
  }
 }

 /**
  * Create a new teleported asset entity
  */
 static async createTeleportedAsset(data: AssetTeleportedProps) {
  let assetTeleported = await AssetTeleported.get(data.id);

  if (typeof assetTeleported === 'undefined') {
   assetTeleported = AssetTeleported.create(data);
   await assetTeleported.save();
  } else {
   logger.info(
    `Attempted to create new asset teleported entity with existing id ${data.id}`
   );
  }
 }

 /**
  * Find received assets by beneficiary address
  */
 static async findReceivedAssetsByBeneficiary(
  beneficiary: string,
  chain: string
 ) {
  return AssetReceived.getByBeneficiary(beneficiary, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find received assets by commitment
  */
 static async findReceivedAssetByCommitment(commitment: string, chain: string) {
  return AssetReceived.getByCommitment(commitment, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find teleported assets by sender address
  */
 static async findTeleportedAssetsByFrom(from: string, chain: string) {
  return AssetTeleported.getByFrom(from, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find teleported assets by recipient address
  */
 static async findTeleportedAssetsByTo(to: string, chain: string) {
  return AssetTeleported.getByTo(to, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find teleported asset by commitment
  */
 static async findTeleportedAssetByCommitment(
  commitment: string,
  chain: string
 ) {
  return AssetTeleported.getByCommitment(commitment, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Get assets by asset ID and chain
  */
 static async getAssetsByType(assetId: string, chain: string) {
  const [received, teleported] = await Promise.all([
   AssetReceived.getByAssetId(assetId, {
    orderBy: 'chain',
    limit: -1,
   }),
   AssetTeleported.getByAssetId(assetId, {
    orderBy: 'chain',
    limit: -1,
   }),
  ]);
  return { received, teleported };
 }
}
