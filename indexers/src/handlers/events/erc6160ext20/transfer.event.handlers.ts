import assert from "assert";
import { HOST_ADDRESSES } from "../../../constants";
import { HyperBridgeService } from "../../../services/hyperbridge.service";
import { RelayerService } from "../../../services/relayer.service";
import { TransferService } from "../../../services/transfer.service";
import { SupportedChain } from "../../../types";
import { TransferLog } from "../../../types/abi-interfaces/ERC6160Ext20Abi";
import { getEvmChainFromTransaction } from "../../../utils/chain.helpers";

/**
 * Handles the Transfer event from the Fee Token contract
 */
export async function handleTransferEvent(event: TransferLog): Promise<void> {
  assert(event.args, "No handleTransferEvent args");
  logger.info("Handling Transfer event");

  const { args, transactionHash, transaction } = event;
  const { from, to, value } = args;

  const chain: SupportedChain = getEvmChainFromTransaction(transaction);

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
        await HyperBridgeService.updateFeesPayedOut(transfer, chain),
      ]);
    }

    if (HOST_ADDRESSES.includes(to)) {
      await HyperBridgeService.updateTotalTransfersIn(transfer, chain);
    }
  }
}
