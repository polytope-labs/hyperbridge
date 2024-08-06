import assert from "assert";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { TransferService } from "../../../services/transfer.service";
import { TransferLog } from "../../../types/abi-interfaces/ERC6160Ext20Abi";
import StateMachineHelpers from "../../../utils/stateMachine.helpers";
import { GET_HOST_ADDRESSES } from "../../../addresses/state-machine.addresses";

/**
 * Handles the Transfer event from the Fee Token contract
 */
export async function handleTransferEvent(event: TransferLog): Promise<void> {
  assert(event.args, "No handleTransferEvent args");
  const { args, transactionHash, transaction, blockNumber } = event;
  const { from, to, value } = args;
  const HOST_ADDRESSES = GET_HOST_ADDRESSES();

  logger.info(
    `Handling Transfer event: ${JSON.stringify({
      blockNumber,
      transactionHash,
    })}`
  );

  const chain: string =
    StateMachineHelpers.getEvmStateMachineIdFromTransaction(transaction);

  // Only store transfers from/to the Hyperbridge host contracts
  if (HOST_ADDRESSES.includes(from) || HOST_ADDRESSES.includes(to)) {
    const transfer = await TransferService.storeTransfer({
      from,
      to,
      value,
      transactionHash,
      chain,
    });

    if (HOST_ADDRESSES.includes(from)) {
      Promise.all([
        await RelayerService.updateFeesEarned(transfer),
        await HyperBridgeService.handleTransferOutOfHostAccounts(
          transfer,
          chain
        ),
      ]);
    }

    if (HOST_ADDRESSES.includes(to)) {
      await HyperBridgeService.updateTotalTransfersIn(transfer, chain);
    }
  }
}
