import { RelayerStatsPerChain } from '../types';

export class RelayerChainStatsService {
 /*
  * Find the RelayerChainMetrics record for a relayer on a chain, create it if it doesn't exist
  */
 static async findOrCreate(
  relayer_id: string,
  chain: string
 ): Promise<RelayerStatsPerChain> {
  let id = `${relayer_id}-${chain}`;
  let metrics = await RelayerStatsPerChain.get(id);

  if (!metrics) {
   metrics = RelayerStatsPerChain.create({
    id,
    relayerId: relayer_id,
    chain,
    numberOfFailedMessagesDelivered: BigInt(0),
    numberOfSuccessfulMessagesDelivered: BigInt(0),
    gasUsedForFailedMessages: BigInt(0),
    gasUsedForSuccessfulMessages: BigInt(0),
    gasFeeForFailedMessages: BigInt(0),
    gasFeeForSuccessfulMessages: BigInt(0),
    usdGasFeeForFailedMessages: BigInt(0),
    usdGasFeeForSuccessfulMessages: BigInt(0),
    feesEarned: BigInt(0),
   });
   await metrics.save();
  }

  return metrics;
 }

 /**
  * Get stats by fees earned
  */
 static async getByFeesEarned(fees: bigint) {
  return RelayerStatsPerChain.getByFeesEarned(fees, {
   orderBy: 'feesEarned',
   limit: -1,
  });
 }
}
