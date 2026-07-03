import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EthereumLog, EthereumResult } from "@subql/types-ethereum"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"
import { Interface } from "@ethersproject/abi"

// The pre-partial-fills EscrowReleased shape, without the solver. The project ABI carries
// the current signature and subql-cli rejects raw topic hashes in manifest filters, so this
// handler is registered without a topic filter and matches the legacy topic itself.
// Decoded with ethers (not viem) because viem's decoders break inside the VM2 sandbox —
// see the note in `utils/phantom-decode.ts`.
const legacyInterface = new Interface([
	"event EscrowReleased(bytes32 indexed commitment, tuple(bytes32 token, uint256 amount)[] tokens)",
])
const LEGACY_ESCROW_RELEASED_TOPIC = legacyInterface.getEventTopic("EscrowReleased")

export const handleEscrowReleasedEventV3Legacy = wrap(async (event: EthereumLog<EthereumResult>): Promise<void> => {
	// Unfiltered handler: every gateway log lands here, so bail fast on anything
	// that isn't a legacy-shape EscrowReleased.
	if (event.topics[0]?.toLowerCase() !== LEGACY_ESCROW_RELEASED_TOPIC.toLowerCase()) return

	logger.info(`[Intent Gateway V3] Legacy Escrow Released Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, blockHash, logIndex } = event
	const { commitment, tokens } = legacyInterface.decodeEventLog(
		"EscrowReleased",
		event.data,
		event.topics,
	) as unknown as { commitment: string; tokens: { token: string; amount: { toString(): string } }[] }

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Legacy Escrow Released: ${stringify({
			commitment,
		})}, tokens: ${stringify(tokens)}`,
	)

	const releasedTokens = tokens.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))

	await IntentGatewayV3Service.recordEscrowRelease(commitment, undefined, releasedTokens, {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})

	// Same inventory publication as the current handler. This shape names no filler, but it only
	// comes from contracts that finalize on every redeem, so `_filled` holds the beneficiary from
	// this block on. Best-effort: it reads external RPCs, and stale depth is recoverable.
	try {
		const beneficiary = await IntentGatewayV3Service.filledBeneficiary(commitment, blockNumber)
		if (beneficiary) {
			await IntentGatewayV3Service.publishInventoryAfterEscrowRelease({
				provider: beneficiary,
				tokens: releasedTokens,
				timestamp,
				blockNumber,
			})
		}
	} catch (e: any) {
		logger.error(`Failed to publish pool inventory for released escrow ${commitment}: ${e.message}`)
	}
})
