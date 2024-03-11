import {
  assert,
  describe,
  test,
  clearStore,
  beforeAll,
  afterAll
} from "matchstick-as/assembly/index"
import { Bytes, BigInt, Address } from "@graphprotocol/graph-ts"
import { GetRequestEvent } from "../generated/schema"
import { GetRequestEvent as GetRequestEventEvent } from "../generated/EthereumHost/EthereumHost"
import { handleGetRequestEvent } from "../src/ethereum-host"
import { createGetRequestEventEvent } from "./ethereum-host-utils"

// Tests structure (matchstick-as >=0.5.0)
// https://thegraph.com/docs/en/developer/matchstick/#tests-structure-0-5-0

describe("Describe entity assertions", () => {
  beforeAll(() => {
    let source = Bytes.fromI32(1234567890)
    let dest = Bytes.fromI32(1234567890)
    let from = Bytes.fromI32(1234567890)
    let keys = [Bytes.fromI32(1234567890)]
    let nonce = BigInt.fromI32(234)
    let height = BigInt.fromI32(234)
    let timeoutTimestamp = BigInt.fromI32(234)
    let gaslimit = BigInt.fromI32(234)
    let fee = BigInt.fromI32(234)
    let newGetRequestEventEvent = createGetRequestEventEvent(
      source,
      dest,
      from,
      keys,
      nonce,
      height,
      timeoutTimestamp,
      gaslimit,
      fee
    )
    handleGetRequestEvent(newGetRequestEventEvent)
  })

  afterAll(() => {
    clearStore()
  })

  // For more test scenarios, see:
  // https://thegraph.com/docs/en/developer/matchstick/#write-a-unit-test

  test("GetRequestEvent created and stored", () => {
    assert.entityCount("GetRequestEvent", 1)

    // 0xa16081f360e3847006db660bae1c6d1b2e17ec2a is the default address used in newMockEvent() function
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "source",
      "1234567890"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "dest",
      "1234567890"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "from",
      "1234567890"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "keys",
      "[1234567890]"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "nonce",
      "234"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "height",
      "234"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "timeoutTimestamp",
      "234"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "gaslimit",
      "234"
    )
    assert.fieldEquals(
      "GetRequestEvent",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "fee",
      "234"
    )

    // More assert options:
    // https://thegraph.com/docs/en/developer/matchstick/#asserts
  })
})
