import assert from "assert";
import { HyperBridgeService } from "../../services/hyperbridge.service";
import { RelayerService } from "../../services/relayer.service";
import { SupportedChain } from "../../types";
import { PostRequestHandledEvent } from "../../types/contracts/EthereumHostAbi";

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
async function handlePostRequestHandledEvent(
  event: PostRequestHandledEvent,
  chain: SupportedChain,
): Promise<void> {
  assert(event.args, "No handlePostRequestHandledEvent args");

  const log_info = {
    message: "Handling PostRequestHandled event",
    chain,
    event: event,
  };

  logger.debug(JSON.stringify(log_info));

  const { args } = event;
  const { relayer } = args;

  Promise.all([
    await RelayerService.incrementNumberOfPostRequestsHandled(relayer, chain),
    await HyperBridgeService.incrementNumberOfPostRequestsHandled(),
  ]);
}

// Handle the PostRequestHandled event for the Ethereum Sepolia chain
export async function handleEthereumSepoliaPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.ETHEREUM_SEPOLIA);
}

// Handle the PostRequestHandled event for the Base Sepolia chain
export async function handleBaseSepoliaPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.BASE_SEPOLIA);
}

// Handle the PostRequestHandled event for the Optimism Sepolia chain
export async function handleOptimismSepoliaPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.OPTIMISM_SEPOLIA);
}

// Handle the PostRequestHandled event for the Arbitrum Sepolia chain
export async function handleArbitrumSepoliaPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.ARBITRUM_SEPOLIA);
}

// Handle the PostRequestHandled event for the BSC Chapel chain
export async function handleBscChapelPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.BSC_CHAPEL);
}
