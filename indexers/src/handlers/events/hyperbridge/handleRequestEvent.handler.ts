import { SubstrateEvent } from "@subql/types";
import { RequestService } from "../../../services/request.service";
import { Status } from "../../../types";
import { HYPERBRIDGE } from "../../../constants";

export async function handleHyperbridgeRequestEvent(
  event: SubstrateEvent
): Promise<void> {
  logger.info(`Handling ISMP Request Event`);

  const {
    event: {
      data: [dest_chain, source_chain, request_nonce, commitment],
    },
    extrinsic,
    block: {
      timestamp,
      block: {
        header: { number: blockNumber, hash: blockHash },
      },
    },
  } = event;

  let transactionHash = "";
  if (extrinsic) {
    transactionHash = extrinsic.extrinsic.hash.toString();
  }

  await RequestService.updateStatus({
    commitment: commitment.toString(),
    chain: HYPERBRIDGE,
    blockNumber: blockNumber.toString(),
    blockHash: blockHash.toString(),
    blockTimestamp: BigInt(Date.parse(timestamp.toString())),
    status: Status.MESSAGE_RELAYED,
    transactionHash,
  });
}
