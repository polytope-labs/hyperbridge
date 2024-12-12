import {
  FulfilledRequest,
  FulfilledRequestProps,
} from "../types/models/FulfilledRequest";

export class FulfilledRequestService {

  /**
   * Create a new fulfilled request
   */
  static async createFulfilledRequest(data: FulfilledRequestProps) {
    let fulfilledRequest = await FulfilledRequest.get(data.id);

    if (typeof fulfilledRequest === "undefined") {
      fulfilledRequest = FulfilledRequest.create(data);
      await fulfilledRequest.save();
    } else {
      logger.info(
        `Attempted to create new fulfilled request with id ${data.id}, but a fulfilled request already exists with this id`,
      );
    }
  }
}
