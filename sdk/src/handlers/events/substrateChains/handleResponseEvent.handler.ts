import { SubstrateEvent } from '@subql/types';
import { ResponseService } from '../../../services/response.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';

export async function handleSubstrateResponseEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP Response Event`);

 if (!event.extrinsic && event.event.data) return;

 const [source_chain, dest_chain, request_nonce, commitment, req_commitment] =
  event.event.data;

 logger.info(
  `Handling ISMP Response Event: ${JSON.stringify({
   source_chain,
   dest_chain,
   request_nonce,
   commitment,
   req_commitment,
  })}`
 );

 const host = getHostStateMachine(chainId);

 if (isHyperbridge(host)) {
  return;
 }

 await ResponseService.findOrCreate({
  chain: host,
  commitment: commitment.toString(),
  status: Status.SOURCE,
  blockNumber: event.block.block.header.number.toString(),
  blockHash: event.block.block.header.hash.toString(),
  transactionHash: event.extrinsic?.extrinsic.hash.toString() || '',
  blockTimestamp: BigInt(event.block.timestamp!.getTime()),
 });
}
