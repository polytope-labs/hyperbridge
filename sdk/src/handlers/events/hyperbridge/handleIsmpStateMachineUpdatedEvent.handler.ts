import { SubstrateEvent } from "@subql/types";
import assert from "assert";
import { HYPERBRIDGE } from "../../../constants";
import { StateMachineService } from "../../../services/stateMachine.service";
import { extractStateMachineIdFromSubstrateEventData } from "../../../utils/substrate.helpers";

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
        timestamp: timestamp ? (Date.parse(timestamp.toString())) : 0,
        stateMachineId,
        height: Number(latest_height.toString()),
      },
      HYPERBRIDGE
    );
  }
}
