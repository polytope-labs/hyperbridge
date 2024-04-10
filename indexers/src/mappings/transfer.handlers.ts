import { RelayerService } from "../services/relayer.service";
import { TransferService } from "../services/transfer.service";
import { SupportedChain } from "../types";
import { TransferEvent } from "../types/contracts/ERC6160Ext20Abi";

/**
 * Handles the Transfer event from Hyper-usd contract
 */
export async function handleTransferEvent(
  event: TransferEvent,
  network: SupportedChain,
): Promise<void> {
  const log_info = {
    message: "Handling Transfer event",
    event: event,
  };

  logger.debug(JSON.stringify(log_info));

  const { args, transactionHash } = event;
  const { from, to, value } = args;

  const transfer = await TransferService.storeTransfer({
    from,
    to,
    value,
    transactionHash,
    network,
  });

  await RelayerService.updateFeesEarned(transfer);
}

// Handles transfers for the Ethereum sepolia network
export async function handleEthereumSepoliaTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.ETHEREUM_SEPOLIA);
}
