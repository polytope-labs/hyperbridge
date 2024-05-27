import { SubstrateEvent } from "@subql/types";
import { StateMachineService } from "../../../services/stateMachine.service";
import { SupportedChain } from "../../../types";
import assert from "assert";
import { extractStateMachineIdFromSubstrateEventData } from "../../../utils/substrate.helpers";

export async function handleIsmpStateMachineUpdatedEvent(
  event: SubstrateEvent,
): Promise<void> {
  logger.info(`Handling ISMP StateMachineUpdatedEvent: `);

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

  const stateMachineId = extractStateMachineIdFromSubstrateEventData(
    state_machine_id.toString(),
  );

  if (typeof stateMachineId === "undefined") {
    return;
  } else {
    await StateMachineService.createHyperbridgeStateMachineUpdatedEvent(
      {
        transactionHash: `${blockNumber}-${extrinsic.idx}`,
        transactionIndex: extrinsic.idx,
        blockNumber: blockNumber.toNumber(),
        blockHash: blockHash.toString(),
        timestamp: Date.parse(timestamp.toString()),
        stateMachineId,
        height: BigInt(latest_height.toString()),
      },
      SupportedChain.HYPERBRIDGE,
    );
  }
}
