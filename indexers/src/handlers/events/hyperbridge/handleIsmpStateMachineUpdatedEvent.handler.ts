import { SubstrateEvent } from "@subql/types";
import { StateMachineService } from "../../../services/stateMachine.service";
import { SupportedChain } from "../../../types";
import assert from "assert";

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

  await StateMachineService.createHyperbridgeStateMachineUpdatedEvent(
    {
      transactionHash: `${extrinsic.extrinsic.hash}`,
      transactionIndex: extrinsic.idx,
      blockNumber: blockNumber.toNumber(),
      blockHash: blockHash.toString(),
      timestamp: Date.parse(timestamp.toString()),
      stateMachineId: state_machine_id.toString(),
      height: BigInt(latest_height.toString()),
    },
    SupportedChain.HYPERBRIDGE,
  );
}
