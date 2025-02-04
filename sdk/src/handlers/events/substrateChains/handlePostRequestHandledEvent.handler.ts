import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';

export async function handleSubstratePostRequestHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostRequestHandled Event`);

 if (!event.extrinsic && event.event.data) return;

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

 const host = getHostStateMachine(chainId);

 if (isHyperbridge(host)) {
  return;
 }

 await RequestService.updateStatus({
  commitment: commitment.toString(),
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
