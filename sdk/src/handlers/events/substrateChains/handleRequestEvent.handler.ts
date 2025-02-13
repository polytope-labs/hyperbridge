import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 formatChain,
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';
import { HyperBridgeService } from '../../../services/hyperbridge.service';

export async function handleSubstrateRequestEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP Request Event`);

 if (!event.extrinsic && event.event.data) return;

 const [source_chain, dest_chain, request_nonce, commitment] = event.event.data;

 logger.info(
  `Handling ISMP Request Event: ${JSON.stringify({
   source_chain,
   dest_chain,
   request_nonce,
   commitment,
  })}`
 );

 const sourceId = formatChain(source_chain.toString());
 const destId = formatChain(dest_chain.toString());

 logger.info(
  `Chain Ids: ${JSON.stringify({
   sourceId,
   destId,
  })}`
 );

 const host = getHostStateMachine(chainId);

 if (isHyperbridge(host)) {
  return;
 }

 await HyperBridgeService.handlePostRequestOrResponseEventSubstrate(
  host,
  event
 );

 await RequestService.findOrCreate({
  chain: host,
  commitment: commitment.toString(),
  body: undefined,
  dest: destId,
  fee: undefined,
  from: undefined,
  nonce: BigInt(request_nonce.toString()),
  source: sourceId,
  timeoutTimestamp: undefined,
  to: undefined,
  status: Status.SOURCE,
  blockNumber: event.block.block.header.number.toString(),
  blockHash: event.block.block.header.hash.toString(),
  transactionHash: event.extrinsic?.extrinsic.hash.toString() || '',
  blockTimestamp: BigInt(event.block?.timestamp!.getTime()),
 });
}
