import { SubstrateEvent } from '@subql/types';
import { AssetService } from '../../../services/asset.service';
import { extractStateMachineIdFromSubstrateEventData, getChainIdFromEvent } from '../../../utils/substrate.helpers';

export async function handleSubstrateAssetEvent(
 event: SubstrateEvent
): Promise<void> {
 const chainId = getChainIdFromEvent(event);

 const stateMachineId = extractStateMachineIdFromSubstrateEventData(
  event.event.data.toString()
 );

 if (typeof stateMachineId === 'undefined') return;

 const { method, data } = event.event;

 switch (method) {
  case 'AssetTransferred':
   await AssetService.createTeleportedAsset({
    id: data[0].toString(),
    commitment: data[0].toString(),
    amount: BigInt(data[1].toString()),
    assetId: data[2].toString(),
    to: data[3].toString(),
    from: data[4].toString(),
    chain: chainId,
    redeem: false,
   });
   break;

  case 'AssetReceived':
   await AssetService.createReceivedAsset({
    id: data[0].toString(),
    amount: BigInt(data[1].toString()),
    assetId: data[2].toString(),
    beneficiary: data[3].toString(),
    chain: chainId,
    commitment: data[0].toString(),
    from: data[4].toString(),
   });
   break;
 }
}
