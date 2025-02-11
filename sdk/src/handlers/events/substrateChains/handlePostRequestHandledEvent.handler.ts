import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';
import { HyperBridgeService } from '../../../services/hyperbridge.service';

type EventData = {
 commitment: string;
 relayer: string;
};
export async function handleSubstratePostRequestHandledEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP PostRequestHandled Event`);

 if (!event.extrinsic && event.event.data) return;

 const {
  extrinsic,
  block: {
   timestamp,
   block: {
    header: { number: blockNumber, hash: blockHash },
   },
  },
 } = event;

 const eventData = event.event.data[0] as unknown as EventData;
 const relayer_id = eventData.relayer.toString();

 logger.info(
  `Handling ISMP PostRequestHandled Event: ${JSON.stringify({
   data: event.event.data,
  })}`
 );

 const host = getHostStateMachine(chainId);

 let status: Status;

 if (isHyperbridge(host)) {
  status = Status.HYPERBRIDGE_DELIVERED;
 } else {
  status = Status.DESTINATION;
 }

 logger.info(`Updating Hyperbridge chain stats for ${host}`);
 await HyperBridgeService.handlePostRequestOrResponseHandledEvent(
  relayer_id,
  host
 );

 logger.info(
  `Handling ISMP PostRequestHandled Event: ${JSON.stringify({
   commitment: eventData.commitment.toString(),
   chain: host,
   blockNumber: blockNumber,
   blockHash: blockHash,
   blockTimestamp: timestamp,
   status,
   transactionHash: extrinsic?.extrinsic.hash || '',
  })}`
 );

 await RequestService.updateStatus({
  commitment: eventData.commitment.toString(),
  chain: host,
  blockNumber: blockNumber.toString(),
  blockHash: blockHash.toString(),
  blockTimestamp: timestamp
   ? BigInt(Date.parse(timestamp.toString()))
   : BigInt(0),
  status,
  transactionHash: extrinsic?.extrinsic.hash.toString() || '',
 });
}
