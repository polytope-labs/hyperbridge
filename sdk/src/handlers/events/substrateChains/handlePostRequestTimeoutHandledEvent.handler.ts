import { SubstrateEvent } from '@subql/types';
import assert from 'assert';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import { extractStateMachineIdFromSubstrateEventData, getChainIdFromEvent } from '../../../utils/substrate.helpers';
import { HYPERBRIDGE } from '../../../constants';

export async function handleSubstratePostRequestTimeoutHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostRequestTimeoutHandled Event`);

 const chainId = getChainIdFromEvent(event);

 const stateMachineId = extractStateMachineIdFromSubstrateEventData(
  event.event.data.toString()
 );

 if (typeof stateMachineId === 'undefined') return;

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

  const timeoutStatus =
   chainId === HYPERBRIDGE ? Status.HYPERBRIDGE_TIMED_OUT : Status.TIMED_OUT;

 const eventData = data.toJSON();
 const timeoutData = Array.isArray(eventData)
  ? (eventData[0] as { commitment: any; source: any; dest: any })
  : undefined;
 assert(timeoutData);

 await RequestService.updateStatus({
  commitment: timeoutData.commitment.toString(),
  chain: chainId,
  blockNumber: blockNumber.toString(),
  blockHash: blockHash.toString(),
  blockTimestamp: timestamp
   ? BigInt(Date.parse(timestamp.toString()))
   : BigInt(0),
  status: timeoutStatus,
  transactionHash: extrinsic.extrinsic.hash.toString(),
 });
}
