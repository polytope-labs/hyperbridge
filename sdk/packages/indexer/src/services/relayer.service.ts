import { ProtocolParticipant } from "@/configs/src/types/enums"
import { Relayer, Transfer } from "@/configs/src/types/models"
import { RelayerChainStatsService } from "@/services/relayerChainStats.service"
// import {
//  HandlePostRequestsTransaction,
//  HandlePostResponsesTransaction,
// } from '@/types/abi-interfaces/HandlerV1Abi';
// import PriceHelper from '@/utils/price.helpers';
// import { GET_ETHEREUM_L2_STATE_MACHINES } from '@/addresses/state-machine.addresses';

export class RelayerService {
	/**
	 * Find a relayer by its id or create a new one if it doesn't exist
	 */
	static async findOrCreate(relayer_id: string, chain: string, timestamp: bigint): Promise<Relayer> {
		let relayer = await Relayer.get(relayer_id)

		if (typeof relayer === "undefined") {
			relayer = Relayer.create({
				id: relayer_id,
				lastUpdatedAt: timestamp,
			})

			await relayer.save()
		}

		return relayer
	}

	/**
	 * Update the total fees earned by a relayer
	 * Fees earned by a relayer == Sum of all transfers to the relayer from the hyperbridge host address
	 */
	static async updateFeesEarned(transfer: Transfer, timestamp: bigint): Promise<void> {
		const relayer = await this.findOrCreate(transfer.to, transfer.chain, timestamp)
		const relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer.id, transfer.chain)

		relayer_chain_stats.feesEarned += transfer.amount
		relayer.lastUpdatedAt = timestamp

		relayer.save()
		relayer_chain_stats.save()
	}

	/**
	 * Update message delivered by the relayer
	 * @param relayer_id The relayer address
	 * @param chain The chain identifier
	 */
	static async updateMessageDelivered(relayer_id: string, chain: string, timestamp: bigint): Promise<void> {
		const relayer = await this.findOrCreate(relayer_id, chain, timestamp)
		const relayer_chain_stats = await RelayerChainStatsService.findOrCreate(relayer.id, chain)

		relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1)
		relayer.lastUpdatedAt = timestamp

		await relayer.save()
		await relayer_chain_stats.save()
	}

	//  /**
	//   * Computes relayer specific stats from the handlePostRequest/handlePostResponse transactions on the handlerV1 contract
	//   */
	//  static async handlePostRequestOrResponseTransaction(
	//   chain: string,
	//   transaction: HandlePostRequestsTransaction | HandlePostResponsesTransaction
	//  ): Promise<void> {
	//   const { from: relayer_id, hash: transaction_hash } = transaction;
	//   const receipt = await transaction.receipt();
	//   const { status, gasUsed, effectiveGasPrice } = receipt;

	//   const nativeCurrencyPrice = await PriceHelper.getNativeCurrencyPrice(chain);

	//   let gasFee = BigInt(effectiveGasPrice) * BigInt(gasUsed);

	//   // Add the L1 Gas Used for L2 chains
	//   if (GET_ETHEREUM_L2_STATE_MACHINES().includes(chain)) {
	//    if (!(receipt as any).l1Fee) {
	//     logger.error(
	//      `Could not find l1Fee in transaction receipt: ${JSON.stringify({
	//       chain,
	//       transactionHash: transaction.hash,
	//      })}`
	//     );
	//    }
	//    const l1Fee = BigInt((receipt as any).l1Fee ?? 0);
	//    gasFee += l1Fee;
	//   }

	//   const _gasFeeInEth = Number(gasFee) / Number(BigInt(10 ** 18));
	//   const usdFee = (gasFee * nativeCurrencyPrice) / BigInt(10 ** 18);

	//   logger.info(
	//    `Handling PostRequest/PostResponse Transaction Relayer Update: ${JSON.stringify(
	//     {
	//      relayer_id,
	//      chain,
	//      transaction_hash,
	//      status,
	//      gasUsed,
	//      gasFee: _gasFeeInEth,
	//      usdFee,
	//     }
	//    )}`
	//   );

	//   try {
	//    let relayer = await RelayerService.findOrCreate(relayer_id, chain);
	//    let relayer_chain_stats = await RelayerChainStatsService.findOrCreate(
	//     relayer_id,
	//     chain
	//    );

	//    if (status === true) {
	//     relayer_chain_stats.numberOfSuccessfulMessagesDelivered += BigInt(1);
	//     relayer_chain_stats.gasUsedForSuccessfulMessages += BigInt(gasUsed);
	//     relayer_chain_stats.gasFeeForSuccessfulMessages += BigInt(gasFee);
	//     relayer_chain_stats.usdGasFeeForSuccessfulMessages += usdFee;
	//     await RewardPointsService.assignRewardToRelayer({
	//      chain,
	//      is_success: true,
	//      earnerType: ProtocolParticipant.RELAYER,
	//      relayer_address: relayer_id,
	//      transaction_hash,
	//     });
	//    } else {
	//     relayer_chain_stats.numberOfFailedMessagesDelivered += BigInt(1);
	//     relayer_chain_stats.gasUsedForFailedMessages += BigInt(gasUsed);
	//     relayer_chain_stats.gasFeeForFailedMessages += BigInt(gasFee);
	//     relayer_chain_stats.usdGasFeeForFailedMessages += usdFee;
	//     await RewardPointsService.assignRewardToRelayer({
	//      chain,
	//      is_success: false,
	//      earnerType: ProtocolParticipant.RELAYER,
	//      relayer_address: relayer_id,
	//      transaction_hash,
	//     });
	//    }

	//    await relayer.save();
	//    await relayer_chain_stats.save();

	//    logger.info(
	//     `Relayer: ${relayer_id} updated successfully for chain: ${chain}`
	//    );
	//   } catch (error) {
	//    logger.error(
	//     `Error while handling PostRequest/PostResponse transaction relayer updates: ${JSON.stringify(
	//      error
	//     )}`
	//    );
	//   }
	//  }
}
