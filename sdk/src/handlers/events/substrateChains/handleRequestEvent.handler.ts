import { SubstrateEvent } from '@subql/types';
import { RequestService } from '../../../services/request.service';
import { Status } from '../../../types';
import {
 getHostStateMachine,
 isHyperbridge,
} from '../../../utils/substrate.helpers';

export async function handleSubstrateRequestEvent(
 event: SubstrateEvent
): Promise<void> {
 logger.info(`Handling ISMP Request Event`);

 if (!event.extrinsic && event.event.data) return;

 const [source_chain, dest_chain, request_nonce, commitment] = event.event.data;

 const host = getHostStateMachine(chainId);

 if (isHyperbridge(host)) {
  return;
 }

 await RequestService.findOrCreate({
  chain: host,
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
