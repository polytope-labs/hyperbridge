import { BigInt } from "@graphprotocol/graph-ts";
import { TransferTotal } from "../../generated/schema";

const ID = "1";
const BIGINT_ZERO = BigInt.fromString("0");

export function getTransferTotal(): TransferTotal {
  let entity = TransferTotal.load(ID);

  if (entity === null) {
    entity = new TransferTotal(ID);
    entity.totalAmount = BIGINT_ZERO;
    entity.save();
  }
  return entity;
}

export function updateTransferTotal(eventValue: BigInt): TransferTotal {
  let entity = getTransferTotal();
  const oldTotalAmount = entity.totalAmount;
  entity.totalAmount = oldTotalAmount.plus(eventValue);
  entity.save();
  return entity;
}
