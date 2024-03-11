import {
  assert,
  describe,
  test,
  clearStore,
  beforeAll,
  afterAll
} from "matchstick-as/assembly/index"
import { BigInt } from "@graphprotocol/graph-ts"
import { StateMachineUpdated } from "../generated/schema"
import { StateMachineUpdated as StateMachineUpdatedEvent } from "../generated/HandlerV1/HandlerV1"
import { handleStateMachineUpdated } from "../src/handler-v-1"
import { createStateMachineUpdatedEvent } from "./handler-v-1-utils"

// Tests structure (matchstick-as >=0.5.0)
// https://thegraph.com/docs/en/developer/matchstick/#tests-structure-0-5-0

describe("Describe entity assertions", () => {
  beforeAll(() => {
    let stateMachineId = BigInt.fromI32(234)
    let height = BigInt.fromI32(234)
    let newStateMachineUpdatedEvent = createStateMachineUpdatedEvent(
      stateMachineId,
      height
    )
    handleStateMachineUpdated(newStateMachineUpdatedEvent)
  })

  afterAll(() => {
    clearStore()
  })

  // For more test scenarios, see:
  // https://thegraph.com/docs/en/developer/matchstick/#write-a-unit-test

  test("StateMachineUpdated created and stored", () => {
    assert.entityCount("StateMachineUpdated", 1)

    // 0xa16081f360e3847006db660bae1c6d1b2e17ec2a is the default address used in newMockEvent() function
    assert.fieldEquals(
      "StateMachineUpdated",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "stateMachineId",
      "234"
    )
    assert.fieldEquals(
      "StateMachineUpdated",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "height",
      "234"
    )

    // More assert options:
    // https://thegraph.com/docs/en/developer/matchstick/#asserts
  })
})
