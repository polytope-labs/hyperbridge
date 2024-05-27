import { SubstrateEvent } from "@subql/types";
import assert from "assert";
import { ResponseService } from "../../../services/response.service";
import { ResponseStatus, SupportedChain } from "../../../types";

export async function handleHyperbridgeResponseEvent(
  event: SubstrateEvent,
): Promise<void> {
  logger.info(`Handling ISMP Response Event`);

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

  await ResponseService.updateStatus({
    commitment: commitment.toString(),
    chain: SupportedChain.HYPERBRIDGE,
    blockNumber: blockNumber.toString(),
    blockHash: blockHash.toString(),
    blockTimestamp: BigInt(Date.parse(timestamp.toString())),
    status: ResponseStatus.MESSAGE_RELAYED,
    transactionHash: extrinsic.extrinsic.hash.toString(),
  });
}
