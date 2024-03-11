import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import { TransferPairTotal } from "../../generated/schema";

const BIGINT_ZERO = BigInt.fromString("0");

export function getTransferPairTotal(fromAddress: Bytes, toAddress: Bytes, pairId: string): TransferPairTotal {
  let entity = TransferPairTotal.load(pairId);

  if (entity === null) {
    entity = new TransferPairTotal(pairId);
    entity.from = fromAddress;
    entity.to = toAddress;
    entity.totalAmount = BIGINT_ZERO;
    entity.save();
  }
  return entity;
}

export function updateTransferPairTotal(fromAddress: Bytes, toAddress: Bytes, eventValue: BigInt): TransferPairTotal {
    let fromAddressHex = fromAddress.toHex().toLowerCase();
    let toAddressHex = toAddress.toHex().toLowerCase();
    let pairId = fromAddressHex + "-" + toAddressHex;

    let entity = getTransferPairTotal(fromAddress, toAddress, pairId);

    const oldTotalAmount = entity.totalAmount;
    entity.from = fromAddress;
    entity.to = toAddress;
    entity.totalAmount = oldTotalAmount.plus(eventValue);
    entity.save();
    return entity;
}
