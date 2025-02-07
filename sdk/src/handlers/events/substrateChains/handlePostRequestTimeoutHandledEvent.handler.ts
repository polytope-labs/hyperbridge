import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';

export async function handleSubstratePostRequestTimeoutHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostRequestTimeoutHandled Event`);

 const host = getHostStateMachine(chainId);

 if (!event.extrinsic) return;

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

 const timeoutStatus = isHyperbridge(host)
  ? Status.HYPERBRIDGE_TIMED_OUT
  : Status.TIMED_OUT;

 const eventData = data.toJSON();
 const timeoutData = Array.isArray(eventData)
  ? (eventData[0] as { commitment: any; source: any; dest: any })
  : undefined;

 if (!timeoutData) return;

 await RequestService.updateStatus({
  commitment: timeoutData.commitment.toString(),
  chain: host,
  blockNumber: blockNumber.toString(),
  blockHash: blockHash.toString(),
  blockTimestamp: timestamp
   ? BigInt(Date.parse(timestamp.toString()))
   : BigInt(0),
  status: timeoutStatus,
  transactionHash: extrinsic.extrinsic.hash.toString(),
 });
}
