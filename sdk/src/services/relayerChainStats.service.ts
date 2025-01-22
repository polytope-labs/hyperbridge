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
  * Get stats by number of successful messages delivered
  */
 static async getBySuccessfulMessages(count: bigint) {
  return RelayerStatsPerChain.getByNumberOfSuccessfulMessagesDelivered(count, {
   orderBy: 'numberOfSuccessfulMessagesDelivered',
   limit: -1,
  });
 }

 /**
  * Get stats by number of failed messages
  */
 static async getByFailedMessages(count: bigint) {
  return RelayerStatsPerChain.getByNumberOfFailedMessagesDelivered(count, {
   orderBy: 'numberOfFailedMessagesDelivered',
   limit: -1,
  });
 }

 /**
  * Get stats by gas used for successful messages
  */
 static async getByGasUsedForSuccess(gasUsed: bigint) {
  return RelayerStatsPerChain.getByGasUsedForSuccessfulMessages(gasUsed, {
   orderBy: 'gasUsedForSuccessfulMessages',
   limit: -1,
  });
 }

 /**
  * Get stats by gas used for failed messages
  */
 static async getByGasUsedForFailures(gasUsed: bigint) {
  return RelayerStatsPerChain.getByGasUsedForFailedMessages(gasUsed, {
   orderBy: 'gasUsedForFailedMessages',
   limit: -1,
  });
 }

 /**
  * Get stats by gas fees for successful messages
  */
 static async getByGasFeeForSuccess(gasFee: bigint) {
  return RelayerStatsPerChain.getByGasFeeForSuccessfulMessages(gasFee, {
   orderBy: 'gasFeeForSuccessfulMessages',
   limit: -1,
  });
 }

 /**
  * Get stats by gas fees for failed messages
  */
 static async getByGasFeeForFailures(gasFee: bigint) {
  return RelayerStatsPerChain.getByGasFeeForFailedMessages(gasFee, {
   orderBy: 'gasFeeForFailedMessages',
   limit: -1,
  });
 }

 /**
  * Get stats by USD gas fees for successful messages
  */
 static async getByUsdGasFeeForSuccess(usdGasFee: bigint) {
  return RelayerStatsPerChain.getByUsdGasFeeForSuccessfulMessages(usdGasFee, {
   orderBy: 'usdGasFeeForSuccessfulMessages',
   limit: -1,
  });
 }

 /**
  * Get stats by USD gas fees for failed messages
  */
 static async getByUsdGasFeeForFailures(usdGasFee: bigint) {
  return RelayerStatsPerChain.getByUsdGasFeeForFailedMessages(usdGasFee, {
   orderBy: 'usdGasFeeForFailedMessages',
   limit: -1,
  });
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
