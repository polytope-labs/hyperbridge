import assert from "assert";
import { SupportedChain } from "../../../types";
import { StateMachineUpdatedLog } from "../../../types/abi-interfaces/EthereumHostAbi";
import { EvmHostEventsService } from "../../../services/evmHostEvents.service";
import {
  SubstrateExtrinsic,
  SubstrateEvent,
  SubstrateBlock,
} from "@subql/types";

/**
 * Handle the StateMachineUpdated event
 */
export async function handleIsmpStateMachineUpdatedEvent(
  event: SubstrateEvent,
): Promise<void> {
  //@Todo - Implement this function
  // assert(event.args, "No handleStateMachineUpdatedEvent args");
  // const {
  //   blockHash,
  //   blockNumber,
  //   transactionHash,
  //   transactionIndex,
  //   block,
  //   transaction,
  //   args,
  // } = event;
  // const { stateMachineId, height } = args;
  // const chain: SupportedChain = getEvmChainFromTransaction(transaction);
  // await EvmHostEventsService.createStateMachineUpdatedEvent(
  //   {
  //     transactionHash,
  //     transactionIndex,
  //     blockHash,
  //     blockNumber,
  //     timestamp: Number(block.timestamp),
  //     stateMachineId: stateMachineId,
  //     height: height.toBigInt(),
  //   },
  //   chain,
  // );

  logger.info(
    `New StateMachineUpdated event found at block ${event.block.block.header.number.toString()}`,
  );

  logger.info(JSON.stringify(event.event.data, null, 2));
}
