import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import { PostRequestHandled } from "../../../generated/schema";

export function findOrCreatePostRequestHandled(ID: Bytes): PostRequestHandled {
  let entity = PostRequestHandled.load(ID);
  if (entity === null) {
    entity = new PostRequestHandled(ID);
  }
  return entity;
}
