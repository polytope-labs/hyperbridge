import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 extractStateMachineIdFromSubstrateEventData,
 getHostStateMachine,
} from '../../../utils/substrate.helpers';
import { HYPERBRIDGE } from '../../../constants';

export async function handleSubstrateRequestEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP Request Event`);

 const [source_chain, dest_chain, request_nonce, commitment] = event.event.data;

 const stateMachineId = extractStateMachineIdFromSubstrateEventData(
  event.event.data.toString()
 );

 if (typeof stateMachineId === 'undefined') return;

 const host = getHostStateMachine(chainId);

 if (host !== HYPERBRIDGE) {
  await RequestService.findOrCreate({
   chain: chainId,
   commitment: commitment.toString(),
   body: undefined,
   dest: dest_chain.toString(),
   fee: undefined,
   from: undefined,
   nonce: BigInt(request_nonce.toString()),
   source: source_chain.toString(),
   timeoutTimestamp: undefined,
   to: undefined,
   status: Status.SOURCE,
   blockNumber: event.block.block.header.number.toString(),
   blockHash: event.block.block.header.hash.toString(),
   transactionHash: event.extrinsic?.extrinsic.hash.toString() || '',
   blockTimestamp: BigInt(event.block?.timestamp!.getTime()),
  });
 }
}
