import {
 FulfilledRequest,
 FulfilledRequestProps,
} from '../types/models/FulfilledRequest';

export class FulfilledRequestService {
 /**
  * Create a new fulfilled request
  */
 static async createFulfilledRequest(data: FulfilledRequestProps) {
  let fulfilledRequest = await FulfilledRequest.get(data.id);

  if (typeof fulfilledRequest === 'undefined') {
   fulfilledRequest = FulfilledRequest.create(data);
   await fulfilledRequest.save();
  } else {
   logger.info(
    `Attempted to create new fulfilled request with id ${data.id}, but a fulfilled request already exists with this id`
   );
  }
 }

 /**
  * Find fulfilled requests by asset ID
  */
 static async findByAssetId(assetId: string) {
  return FulfilledRequest.getByAssetId(assetId, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find fulfilled requests by bidder address
  */
 static async findByBidder(bidder: string) {
  return FulfilledRequest.getByBidder(bidder, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find fulfilled requests by amount
  */
 static async findByAmount(amount: bigint) {
  return FulfilledRequest.getByAmount(amount, {
   orderBy: 'chain',
   limit: -1,
  });
 }

 /**
  * Find fulfilled requests by chain
  */
 static async findByChain(chain: string) {
  return FulfilledRequest.getByChain(chain, {
   orderBy: 'chain',
   limit: -1,
  });
 }
}
