import { SubstrateEvent } from "@subql/types";
import assert from "assert";
import { HYPERBRIDGE } from "../../../constants";
import { RequestService } from "../../../services/request.service";
import { Status } from "../../../types";

export async function handleHyperbridgePostRequestTimeoutHandledEvent(
  event: SubstrateEvent
): Promise<void> {
  logger.info(`Handling ISMP PostRequestTimeoutHandled Event`);

  assert(event.extrinsic);
  const {
    event: { data },
    extrinsic,
    block: {
      timestamp,
      block: {
        header: { number: blockNumber, hash: blockHash },
      },
    },
  } = event;

  const eventData = data.toJSON();
  const timeoutData = Array.isArray(eventData)
    ? (eventData[0] as { commitment: any; source: any; dest: any })
    : undefined;
  assert(timeoutData);

  await RequestService.updateStatus({
    commitment: timeoutData.commitment.toString(),
    chain: HYPERBRIDGE,
    blockNumber: blockNumber.toString(),
    blockHash: blockHash.toString(),
    blockTimestamp: timestamp ? BigInt(Date.parse(timestamp.toString())) : BigInt(0),
    status: Status.TIMED_OUT,
    transactionHash: extrinsic.extrinsic.hash.toString(),
  });
}
