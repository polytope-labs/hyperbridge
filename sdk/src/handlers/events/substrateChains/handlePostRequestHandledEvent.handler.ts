import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 extractStateMachineIdFromSubstrateEventData,
 getChainIdFromEvent,
} from '../../../utils/substrate.helpers';
import { HYPERBRIDGE } from '../../../constants';

export async function handleSubstratePostRequestHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostRequestHandled Event`);

 const chainId = getChainIdFromEvent(event);
 const stateMachineId = extractStateMachineIdFromSubstrateEventData(
  event.event.data.toString()
 );

 if (typeof stateMachineId === 'undefined') return;

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

  const status =
   chainId === HYPERBRIDGE ? Status.MESSAGE_RELAYED : Status.DEST;

 await RequestService.updateStatus({
  commitment: commitment.toString(),
  chain: chainId,
  blockNumber: blockNumber.toString(),
  blockHash: blockHash.toString(),
  blockTimestamp: timestamp
   ? BigInt(Date.parse(timestamp.toString()))
   : BigInt(0),
  status,
  transactionHash: extrinsic?.extrinsic.hash.toString() || '',
 });
}
