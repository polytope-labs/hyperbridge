import { SubstrateEvent } from '@subql/types';
import { ResponseService } from '../../../services/response.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';

export async function handleSubstratePostResponseHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostResponseHandled Event`);

 if (!event.extrinsic && event.event.data) return;

 const {
  event: {
   data: [
    dest_chain,
    source_chain,
    request_nonce,
    commitment,
    response_commitment,
   ],
  },
  extrinsic,
  block: {
   timestamp,
   block: {
    header: { number: blockNumber, hash: blockHash },
   },
  },
 } = event;

 const host = getHostStateMachine(chainId);

  if (isHyperbridge(host)) {
   return;
  }

 await ResponseService.updateStatus({
  commitment: response_commitment.toString(),
  chain: host,
  blockNumber: blockNumber.toString(),
  blockHash: blockHash.toString(),
  blockTimestamp: timestamp
   ? BigInt(Date.parse(timestamp.toString()))
   : BigInt(0),
  status: Status.DESTINATION,
  transactionHash: extrinsic?.extrinsic.hash.toString() || '',
 });
}
