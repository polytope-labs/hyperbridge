import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import { PostRequestHandledCount } from "../../../generated/schema";

const ID = "1";
const BIGINT_ONE = BigInt.fromString("1");
const BIGINT_ZERO = BigInt.fromString("0");

export function getPostRequestHandledCount(): PostRequestHandledCount {
    let entity = PostRequestHandledCount.load(ID);
    if (entity === null) {
      entity = new PostRequestHandledCount(ID);
      entity.value = BIGINT_ZERO;
      entity.save();
    }
    return entity;
  }
  
  export function incrementPostRequestHandledCount(): PostRequestHandledCount {
    let entity = getPostRequestHandledCount();
    const oldValue = entity.value;
    entity.value = oldValue.plus(BIGINT_ONE);
    entity.save();
    return entity;
  }