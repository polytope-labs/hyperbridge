import assert from "assert";
import { HOST_ADDRESSES } from "../../constants";
import { HyperBridgeService } from "../../services/hyperbridge.service";
import { RelayerService } from "../../services/relayer.service";
import { TransferService } from "../../services/transfer.service";
import { SupportedChain } from "../../types";
import { TransferEvent } from "../../types/contracts/ERC6160Ext20Abi";

/**
 * Handles the Transfer event from the Fee Token contract
 */
async function handleTransferEvent(
  event: TransferEvent,
  chain: SupportedChain,
): Promise<void> {
  assert(event.args, "No handleTransferEvent args");
  logger.info("Handling Transfer event");

  const { args, transactionHash } = event;
  const { from, to, value } = args;

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

// Handles transfers for the Ethereum sepolia network
export async function handleEthereumSepoliaTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.ETHEREUM_SEPOLIA);
}

// Handles transfers for the Base sepolia network
export async function handleBaseSepoliaTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.BASE_SEPOLIA);
}

// Handles transfers for the Optimism sepolia network
export async function handleOptimismSepoliaTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.OPTIMISM_SEPOLIA);
}

// Handles transfers for the Arbitrum sepolia network
export async function handleArbitrumSepoliaTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.ARBITRUM_SEPOLIA);
}

// Handles transfers for the BSC Chapel network
export async function handleBscChapelTransferEvent(
  event: TransferEvent,
): Promise<void> {
  await handleTransferEvent(event, SupportedChain.BSC_CHAPEL);
}
