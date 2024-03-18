import { BigInt } from "@graphprotocol/graph-ts";
import { RelayerPostRequestHandledCount } from "../../../generated/schema";

const BIGINT_ONE = BigInt.fromString("1");
const BIGINT_ZERO = BigInt.fromString("0");

export function getRelayerPostRequestHandledCount(addr: string): RelayerPostRequestHandledCount {
  let entity = RelayerPostRequestHandledCount.load(addr);
  if (entity === null) {
    entity = new RelayerPostRequestHandledCount(addr);
    entity.value = BIGINT_ZERO;
    entity.save();
  }
  return entity;
}

export function incrementRelayerPostRequestHandledCount(addr: string): RelayerPostRequestHandledCount {
  let entity = getRelayerPostRequestHandledCount(addr);
  const oldValue = entity.value;
  entity.value = oldValue.plus(BIGINT_ONE);
  entity.save();
  return entity;
}
