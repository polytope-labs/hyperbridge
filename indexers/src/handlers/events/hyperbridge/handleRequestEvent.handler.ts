import { SubstrateEvent } from "@subql/types";
import { RequestService } from "../../../services/request.service";
import { Status, SupportedChain } from "../../../types";
import assert from "assert";

export async function handleHyperbridgeRequestEvent(
  event: SubstrateEvent,
): Promise<void> {
  logger.info(`Handling ISMP Request Event`);
  assert(event.extrinsic);

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

  await RequestService.updateStatus({
    commitment: commitment.toString(),
    chain: SupportedChain.HYPERBRIDGE,
    blockNumber: blockNumber.toString(),
    blockHash: blockHash.toString(),
    blockTimestamp: BigInt(Date.parse(timestamp.toString())),
    status: Status.MESSAGE_RELAYED,
    transactionHash: extrinsic.extrinsic.hash.toString(),
  });
}
