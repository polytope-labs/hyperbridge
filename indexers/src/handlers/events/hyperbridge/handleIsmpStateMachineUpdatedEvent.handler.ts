import { SubstrateEvent } from "@subql/types";
import { StateMachineService } from "../../../services/stateMachine.service";
import assert from "assert";
import { extractStateMachineIdFromSubstrateEventData } from "../../../utils/substrate.helpers";
import { HYPERBRIDGE } from "../../../constants";

export async function handleIsmpStateMachineUpdatedEvent(
  event: SubstrateEvent
): Promise<void> {
  const {
    event: {
      data: [state_machine_id, latest_height],
    },
    extrinsic,
    block: {
      timestamp,
      block: {
        header: { number: blockNumber, hash: blockHash },
      },
    },
  } = event;

  assert(extrinsic);
  logger.info(
    `Handling ISMP StateMachineUpdatedEvent. Block Number: ${blockNumber}`
  );

  const stateMachineId = extractStateMachineIdFromSubstrateEventData(
    state_machine_id.toString()
  );

  if (typeof stateMachineId === "undefined") {
    return;
  } else {
    await StateMachineService.createHyperbridgeStateMachineUpdatedEvent(
      {
        transactionHash: `${extrinsic.extrinsic.hash}`,
        transactionIndex: extrinsic.idx,
        blockNumber: blockNumber.toNumber(),
        blockHash: blockHash.toString(),
        timestamp: Date.parse(timestamp.toString()),
        stateMachineId,
        height: Number(latest_height.toString()),
      },
      HYPERBRIDGE
    );
  }
}
