import { StateMachineUpdated as StateMachineUpdatedEvent } from "../generated/HandlerV1/HandlerV1"
import { StateMachineUpdated } from "../generated/schema"

export function handleStateMachineUpdated(
  event: StateMachineUpdatedEvent
): void {
  let entity = new StateMachineUpdated(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  )
  entity.stateMachineId = event.params.stateMachineId
  entity.height = event.params.height

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}
