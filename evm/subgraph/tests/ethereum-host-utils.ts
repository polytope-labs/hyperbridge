import { newMockEvent } from "matchstick-as"
import { ethereum, Bytes, BigInt, Address } from "@graphprotocol/graph-ts"
import {
  GetRequestEvent,
  GetRequestHandled,
  PostRequestEvent,
  PostRequestHandled,
  PostResponseEvent,
  PostResponseHandled
} from "../generated/EthereumHost/EthereumHost"

export function createGetRequestEventEvent(
  source: Bytes,
  dest: Bytes,
  from: Bytes,
  keys: Array<Bytes>,
  nonce: BigInt,
  height: BigInt,
  timeoutTimestamp: BigInt,
  gaslimit: BigInt,
  fee: BigInt
): GetRequestEvent {
  let getRequestEventEvent = changetype<GetRequestEvent>(newMockEvent())

  getRequestEventEvent.parameters = new Array()

  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("source", ethereum.Value.fromBytes(source))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("dest", ethereum.Value.fromBytes(dest))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("from", ethereum.Value.fromBytes(from))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("keys", ethereum.Value.fromBytesArray(keys))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("nonce", ethereum.Value.fromUnsignedBigInt(nonce))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("height", ethereum.Value.fromUnsignedBigInt(height))
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam(
      "timeoutTimestamp",
      ethereum.Value.fromUnsignedBigInt(timeoutTimestamp)
    )
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam(
      "gaslimit",
      ethereum.Value.fromUnsignedBigInt(gaslimit)
    )
  )
  getRequestEventEvent.parameters.push(
    new ethereum.EventParam("fee", ethereum.Value.fromUnsignedBigInt(fee))
  )

  return getRequestEventEvent
}

export function createGetRequestHandledEvent(
  commitment: Bytes,
  relayer: Address
): GetRequestHandled {
  let getRequestHandledEvent = changetype<GetRequestHandled>(newMockEvent())

  getRequestHandledEvent.parameters = new Array()

  getRequestHandledEvent.parameters.push(
    new ethereum.EventParam(
      "commitment",
      ethereum.Value.fromFixedBytes(commitment)
    )
  )
  getRequestHandledEvent.parameters.push(
    new ethereum.EventParam("relayer", ethereum.Value.fromAddress(relayer))
  )

  return getRequestHandledEvent
}

export function createPostRequestEventEvent(
  source: Bytes,
  dest: Bytes,
  from: Bytes,
  to: Bytes,
  nonce: BigInt,
  timeoutTimestamp: BigInt,
  data: Bytes,
  gaslimit: BigInt,
  fee: BigInt
): PostRequestEvent {
  let postRequestEventEvent = changetype<PostRequestEvent>(newMockEvent())

  postRequestEventEvent.parameters = new Array()

  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("source", ethereum.Value.fromBytes(source))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("dest", ethereum.Value.fromBytes(dest))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("from", ethereum.Value.fromBytes(from))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("to", ethereum.Value.fromBytes(to))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("nonce", ethereum.Value.fromUnsignedBigInt(nonce))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam(
      "timeoutTimestamp",
      ethereum.Value.fromUnsignedBigInt(timeoutTimestamp)
    )
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("data", ethereum.Value.fromBytes(data))
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam(
      "gaslimit",
      ethereum.Value.fromUnsignedBigInt(gaslimit)
    )
  )
  postRequestEventEvent.parameters.push(
    new ethereum.EventParam("fee", ethereum.Value.fromUnsignedBigInt(fee))
  )

  return postRequestEventEvent
}

export function createPostRequestHandledEvent(
  commitment: Bytes,
  relayer: Address
): PostRequestHandled {
  let postRequestHandledEvent = changetype<PostRequestHandled>(newMockEvent())

  postRequestHandledEvent.parameters = new Array()

  postRequestHandledEvent.parameters.push(
    new ethereum.EventParam(
      "commitment",
      ethereum.Value.fromFixedBytes(commitment)
    )
  )
  postRequestHandledEvent.parameters.push(
    new ethereum.EventParam("relayer", ethereum.Value.fromAddress(relayer))
  )

  return postRequestHandledEvent
}

export function createPostResponseEventEvent(
  source: Bytes,
  dest: Bytes,
  from: Bytes,
  to: Bytes,
  nonce: BigInt,
  timeoutTimestamp: BigInt,
  data: Bytes,
  gaslimit: BigInt,
  response: Bytes,
  resGaslimit: BigInt,
  resTimeoutTimestamp: BigInt,
  fee: BigInt
): PostResponseEvent {
  let postResponseEventEvent = changetype<PostResponseEvent>(newMockEvent())

  postResponseEventEvent.parameters = new Array()

  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("source", ethereum.Value.fromBytes(source))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("dest", ethereum.Value.fromBytes(dest))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("from", ethereum.Value.fromBytes(from))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("to", ethereum.Value.fromBytes(to))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("nonce", ethereum.Value.fromUnsignedBigInt(nonce))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam(
      "timeoutTimestamp",
      ethereum.Value.fromUnsignedBigInt(timeoutTimestamp)
    )
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("data", ethereum.Value.fromBytes(data))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam(
      "gaslimit",
      ethereum.Value.fromUnsignedBigInt(gaslimit)
    )
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("response", ethereum.Value.fromBytes(response))
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam(
      "resGaslimit",
      ethereum.Value.fromUnsignedBigInt(resGaslimit)
    )
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam(
      "resTimeoutTimestamp",
      ethereum.Value.fromUnsignedBigInt(resTimeoutTimestamp)
    )
  )
  postResponseEventEvent.parameters.push(
    new ethereum.EventParam("fee", ethereum.Value.fromUnsignedBigInt(fee))
  )

  return postResponseEventEvent
}

export function createPostResponseHandledEvent(
  commitment: Bytes,
  relayer: Address
): PostResponseHandled {
  let postResponseHandledEvent = changetype<PostResponseHandled>(newMockEvent())

  postResponseHandledEvent.parameters = new Array()

  postResponseHandledEvent.parameters.push(
    new ethereum.EventParam(
      "commitment",
      ethereum.Value.fromFixedBytes(commitment)
    )
  )
  postResponseHandledEvent.parameters.push(
    new ethereum.EventParam("relayer", ethereum.Value.fromAddress(relayer))
  )

  return postResponseHandledEvent
}
