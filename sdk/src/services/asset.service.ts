import {
  AssetReceived,
  AssetReceivedProps,
} from "../types/models/AssetReceived";
import {
  AssetTeleported,
  AssetTeleportedProps,
} from "../types/models/AssetTeleported";

export class AssetService {

  /**
   * Create a new received asset entity
   */
  static async createReceivedAsset(data: AssetReceivedProps) {
    let assetReceived = await AssetReceived.get(data.id);

    if (typeof assetReceived === "undefined") {
      assetReceived = AssetReceived.create(data);
      await assetReceived.save();
    } else {
      logger.info(
        `Attempted to create new asset received entity with existing id ${data.id}`,
      );
    }
  }

  /**
   * Create a new teleported asset entity
   */
  static async createTeleportedAsset(data: AssetTeleportedProps) {
    let assetTeleported = await AssetTeleported.get(data.id);

    if (typeof assetTeleported === "undefined") {
      assetTeleported = AssetTeleported.create(data);
      await assetTeleported.save();
    } else {
      logger.info(
        `Attempted to create new asset teleported entity with existing id ${data.id}`,
      );
    }
  }
}
