import { BigInt, Bytes } from "@graphprotocol/graph-ts";
import {
  Approval as ApprovalEvent,
  Transfer as TransferEvent,
} from "../generated/GovernableToken/GovernableToken"
import { Approval, Transfer } from "../generated/schema"

import { updateTransferTotal } from "./utils/TransferTotal";
import { updateTransferPairTotal } from "./utils/TransferPairTotal";
import { updateAggregatedTotal } from "./utils/AggregatedTotal";
import { updateInTransferTotal } from "./utils/InTransferTotal";

export function handleApproval(event: ApprovalEvent): void {
  let entity = new Approval(
    event.transaction.hash.concatI32(event.logIndex.toI32()),
  )
  entity.owner = event.params.owner
  entity.spender = event.params.spender
  entity.value = event.params.value

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}

export function handleTransfer(event: TransferEvent): void {
  updateTransferTotal( event.params.value);
  updateTransferPairTotal(event.params.from,event.params.to, event.params.value);

  updateInTransferTotal(event.params.to.toHexString(), event.params.value)

  // const hostAddressString: string = "0x9DF353352b469782AB1B0F2CbBFEC41bF1FDbDb3";
  // const hostAddressBytes: Bytes = Bytes.fromHexString(hostAddressString);

  // updateAggregatedTotal(hostAddressBytes, BigInt.fromI32(0), event.params.value);
  // updateAggregatedTotal(event.params.to, BigInt.fromI32(0), event.params.value);

  let entity = new Transfer(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  )
  entity.from = event.params.from
  entity.to = event.params.to
  entity.value = event.params.value

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}
