import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import { AggregatedTotal } from "../../generated/schema";

// const BIGINT_ZERO = BigInt.fromString("0");

export function updateAggregatedTotal(hostAddress: Bytes, relayerFee: BigInt, transferInValue: BigInt): AggregatedTotal {
    let id = hostAddress.toHexString();
    let entity = AggregatedTotal.load(id);
  
    if (entity == null) {
        entity = new AggregatedTotal(id);
        entity.totalRelayerFees = BigInt.fromI32(0);
        entity.totalTransferredValues = BigInt.fromI32(0);
        entity.hyperbridgeEarnings = BigInt.fromI32(0);
    }
  
    entity.totalRelayerFees = entity.totalRelayerFees.plus(relayerFee);
    entity.totalTransferredValues = entity.totalTransferredValues.plus(transferInValue);

    // Ensure to recalculate netValue whenever we update totals
    entity.hyperbridgeEarnings = entity.totalTransferredValues.minus(entity.totalRelayerFees);
    entity.save();

    return entity;
  }
