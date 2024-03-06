import { BigInt } from "@graphprotocol/graph-ts";
import { InTransferTotal } from "../../generated/schema";

const BIGINT_ZERO = BigInt.fromString("0");

export function getInTransferTotal(toAddress: string): InTransferTotal {
  let entity = InTransferTotal.load(toAddress);

  if (entity === null) {
    entity = new InTransferTotal(toAddress);
    entity.totalAmountTransferredIn = BIGINT_ZERO;
    entity.save();
  }
  return entity;
}

export function updateInTransferTotal(toAddress: string, eventValue: BigInt): InTransferTotal {
    let entity = getInTransferTotal(toAddress);

    const oldTotalAmount = entity.totalAmountTransferredIn;
    entity.totalAmountTransferredIn = oldTotalAmount.plus(eventValue);

    entity.save();
    return entity;
}
