import { newMockEvent } from "matchstick-as"
import { ethereum, BigInt } from "@graphprotocol/graph-ts"
import { StateMachineUpdated } from "../generated/HandlerV1/HandlerV1"

export function createStateMachineUpdatedEvent(
  stateMachineId: BigInt,
  height: BigInt
): StateMachineUpdated {
  let stateMachineUpdatedEvent = changetype<StateMachineUpdated>(newMockEvent())

  stateMachineUpdatedEvent.parameters = new Array()

  stateMachineUpdatedEvent.parameters.push(
    new ethereum.EventParam(
      "stateMachineId",
      ethereum.Value.fromUnsignedBigInt(stateMachineId)
    )
  )
  stateMachineUpdatedEvent.parameters.push(
    new ethereum.EventParam("height", ethereum.Value.fromUnsignedBigInt(height))
  )

  return stateMachineUpdatedEvent
}
