import { Bid } from "../types/models";
import { BidProps } from "../types/models/Bid";
import { BidRefund, BidRefundProps } from "../types/models/BidRefund";
import {
  FulfilledRequest,
  FulfilledRequestProps,
} from "../types/models/FulfilledRequest";

export class BidService {
  /**
   * Create a new bid
   */
  static async createBid(data: BidProps) {
    let bid = await Bid.get(data.id);

    if (typeof bid === "undefined") {
      bid = Bid.create(data);
      await bid.save();
    } else {
      logger.info(
        `Attempted to create new bid with commitment ${data.id}, but a bid already exists with this commitment`,
      );
    }
  }

  /**
   * Create a new bid refund
   */
  static async createBidRefund(data: BidRefundProps) {
    let bidRefund = await BidRefund.get(data.id);

    if (typeof bidRefund === "undefined") {
      bidRefund = BidRefund.create(data);
      await bidRefund.save();
    } else {
      logger.info(
        `Attempted to create new bid refund with commitment ${data.id}, but a refund already exists with this commitment`,
      );
    }
  }

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
