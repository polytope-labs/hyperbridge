import { SubstrateEvent } from '@subql/types';
import assert from 'assert';
import { ResponseService } from '../../../services/response.service';
import { Status } from '../../../types';
import {
 extractStateMachineIdFromSubstrateEventData,
 getHostStateMachine,
} from '../../../utils/substrate.helpers';
import { HYPERBRIDGE } from '../../../constants';

export async function handleSubstrateResponseEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP Response Event`);

 const stateMachineId = extractStateMachineIdFromSubstrateEventData(
  event.event.data.toString()
 );

 if (typeof stateMachineId === 'undefined') return;

 assert(event.extrinsic);
 const [source_chain, dest_chain, request_nonce, commitment, req_commitment] =
  event.event.data;

 const host = getHostStateMachine(chainId);

 if (host !== HYPERBRIDGE) {
  await ResponseService.findOrCreate({
   chain: chainId,
   commitment: commitment.toString(),
   status: Status.DESTINATION,
   blockNumber: event.block.block.header.number.toString(),
   blockHash: event.block.block.header.hash.toString(),
   transactionHash: event.extrinsic?.extrinsic.hash.toString() || '',
   blockTimestamp: BigInt(event.block.timestamp!.getTime()),
  });
 }
}
