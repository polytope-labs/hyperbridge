import { BigInt } from "@graphprotocol/graph-ts";
import { RequestEventFeeTotal } from "../../generated/schema";

const BIGINT_ZERO = BigInt.fromString("0");
const ID = "1";

export function getRequestEventFeeTotal(): RequestEventFeeTotal {
  let entity = RequestEventFeeTotal.load(ID);

  if (entity === null) {
    entity = new RequestEventFeeTotal(ID);
    entity.totalRequestFee = BIGINT_ZERO;
    entity.save();
  }

  return entity;
}

export function updateRequestEventFeeTotal(fee: BigInt): RequestEventFeeTotal {
    let entity = getRequestEventFeeTotal();

    const oldTotalAmount = entity.totalRequestFee;
    entity.totalRequestFee = oldTotalAmount.plus(fee);

    entity.save();

    return entity;
}
