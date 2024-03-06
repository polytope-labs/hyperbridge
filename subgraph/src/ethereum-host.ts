import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import {
  GetRequestEvent as GetRequestEventEvent,
  GetRequestHandled as GetRequestHandledEvent,
  PostRequestEvent as PostRequestEventEvent,
  PostRequestHandled as PostRequestHandledEvent,
  PostResponseEvent as PostResponseEventEvent,
  PostResponseHandled as PostResponseHandledEvent,
} from "../generated/EthereumHost/EthereumHost"
import {
  getPostRequestHandledCount,
  incrementPostRequestHandledCount,
} from "./utils/postRequest/PostRequestHandledCount";

import { findOrCreatePostRequestHandled } from "./utils/postRequest/PostRequestHandled";
import { incrementRelayerPostRequestHandledCount } from "./utils/postRequest/RelayerPostRequestHandledCount";
import { updateAggregatedTotal } from "./utils/AggregatedTotal";
import { updateRequestEventFeeTotal } from "./utils/RequestEventFeeTotal";

import {
  GetRequestEvent,
  GetRequestHandled,
  PostRequestEvent,
  PostRequestHandled,
  PostResponseEvent,
  PostResponseHandled,
} from "../generated/schema"

export function handleGetRequestEvent(event: GetRequestEventEvent): void {
  let entity = new GetRequestEvent(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )
  entity.source = event.params.source
  entity.dest = event.params.dest
  entity.from = event.params.from
  entity.keys = event.params.keys
  entity.nonce = event.params.nonce
  entity.height = event.params.height
  entity.timeoutTimestamp = event.params.timeoutTimestamp
  entity.gaslimit = event.params.gaslimit
  entity.fee = event.params.fee

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handleGetRequestHandled(event: GetRequestHandledEvent): void {
  let entity = new GetRequestHandled(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )
  entity.commitment = event.params.commitment
  entity.relayer = event.params.relayer

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handlePostRequestEvent(event: PostRequestEventEvent): void {
  let entity = new PostRequestEvent(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )

  updateRequestEventFeeTotal(event.params.fee);

  const hostAddressString: string = "0x9DF353352b469782AB1B0F2CbBFEC41bF1FDbDb3";
  const hostAddressBytes: Bytes = Bytes.fromHexString(hostAddressString);

  updateAggregatedTotal(hostAddressBytes, event.params.fee, BigInt.fromI32(0));

  entity.source = event.params.source
  entity.dest = event.params.dest
  entity.from = event.params.from
  entity.to = event.params.to
  entity.nonce = event.params.nonce
  entity.timeoutTimestamp = event.params.timeoutTimestamp
  entity.data = event.params.data
  entity.gaslimit = event.params.gaslimit
  entity.fee = event.params.fee

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handlePostRequestHandled(event: PostRequestHandledEvent): void {
  incrementPostRequestHandledCount();
  incrementRelayerPostRequestHandledCount(event.params.relayer.toHexString());

  const requestHandledCount = getPostRequestHandledCount();

  let entity = findOrCreatePostRequestHandled(event.transaction.hash.concatI32(event.logIndex.toI32()));

  // let entity = new PostRequestHandled(
  //   event.transaction.hash.concatI32(event.logIndex.toI32()),
  // )
  entity.requestIndex = requestHandledCount.value;
  entity.commitment = event.params.commitment
  entity.relayer = event.params.relayer
  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handlePostResponseEvent(event: PostResponseEventEvent): void {
  let entity = new PostResponseEvent(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )
  entity.source = event.params.source
  entity.dest = event.params.dest
  entity.from = event.params.from
  entity.to = event.params.to
  entity.nonce = event.params.nonce
  entity.timeoutTimestamp = event.params.timeoutTimestamp
  entity.data = event.params.data
  entity.gaslimit = event.params.gaslimit
  entity.response = event.params.response
  entity.resGaslimit = event.params.resGaslimit
  entity.resTimeoutTimestamp = event.params.resTimeoutTimestamp
  entity.fee = event.params.fee

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handlePostResponseHandled(
  event: PostResponseHandledEvent,
): void {
  let entity = new PostResponseHandled(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )
  entity.commitment = event.params.commitment
  entity.relayer = event.params.relayer

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}
