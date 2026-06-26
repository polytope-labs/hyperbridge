import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V3_ADDRESSES } from "@/intent-gateway-v3-addresses"
import { TOKEN_SLOT_OVERRIDES } from "@/token-slot-overrides"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { PhantomOrder, PhantomOrderLpBalance, PhantomOrderPriceSnapshot } from "@/configs/src/types"
import { aggregatePhantomBids, type HexString } from "@hyperbridge/sdk/intents-helpers"

// Triggered by PhantomBidWindowExhausted once a phantom order's bid window closes, so every bid is
// already in. Aggregates that single order's bids into one price snapshot. The heavy lifting lives in
// aggregatePhantomBids(); this handler just resolves endpoints and persists the result.
export const handlePhantomOrderPrices = wrap(async (event: SubstrateEvent): Promise<void> => {
	const blockNumber = event.block.block.header.number.toBigInt()
	const blockHash = event.block.block.header.hash.toString()

	const [commitmentData] = event.event.data
	const commitment = commitmentData.toHex()

	const phantom = await PhantomOrder.get(commitment)
	if (!phantom) return

	const snapshotId = `${commitment}-${blockNumber}`
	if (await PhantomOrderPriceSnapshot.get(snapshotId)) return

	let host: string
	try {
		host = getHostStateMachine(chainId)
	} catch (err) {
		logger.warn({ err, chainId }, "Unrecognised host chain for phantom price snapshot, skipping")
		return
	}
	const nodeUrl = replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "")
	if (!nodeUrl) {
		logger.warn({ host }, "No RPC URL configured for Hyperbridge node")
		return
	}

	const evmRpcUrl = replaceWebsocketWithHttp(ENV_CONFIG[phantom.chain] ?? "")
	const gatewayAddress = INTENT_GATEWAY_V3_ADDRESSES[phantom.chain as keyof typeof INTENT_GATEWAY_V3_ADDRESSES]
	if (!evmRpcUrl || !gatewayAddress) return

	let aggregate
	try {
		aggregate = await aggregatePhantomBids({
			nodeUrl,
			evmRpcUrl,
			chain: phantom.chain,
			gatewayAddress,
			commitment,
			inputToken: phantom.tokenA as HexString,
			standardAmount: phantom.standardAmount,
			tokenSlotOverrides: TOKEN_SLOT_OVERRIDES,
			yieldVaults: YIELD_VAULT_ADDRESSES,
			logger,
		})
	} catch (err) {
		logger.warn({ err, commitment }, "Phantom bid aggregation failed")
		return
	}
	if (!aggregate) return

	const blockTimestamp = await getBlockTimestamp(blockHash, host)
	const snapshotTime = timestampToDate(blockTimestamp)

	for (const lp of aggregate.lpBalances) {
		// One row per solver per snapshot so liquidity history is preserved and each balance is
		// attributable to the snapshot whose weighted median it fed.
		await PhantomOrderLpBalance.create({
			id: `${phantom.chain}-${commitment}-${blockNumber}-${lp.solver}`,
			chain: phantom.chain,
			commitment,
			blockNumber,
			solver: lp.solver,
			tokenAddress: lp.tokenAddress,
			balance: lp.balance,
			snapshotTime,
		}).save()
	}

	await PhantomOrderPriceSnapshot.create({
		id: snapshotId,
		commitment,
		tokenA: bytes32ToBytes20(phantom.tokenA),
		tokenB: bytes32ToBytes20(phantom.tokenB),
		// Denormalized from PhantomOrder so a rate (medianPrice / standardAmount) is computable
		// from a single snapshot row without joining back to the order.
		standardAmount: phantom.standardAmount,
		blockNumber,
		lowestPrice: aggregate.lowestPrice,
		highestPrice: aggregate.highestPrice,
		medianPrice: aggregate.medianPrice,
		bidCount: aggregate.bidCount,
		snapshotTime,
	}).save()

	logger.info({ commitment, blockNumber, bidCount: aggregate.bidCount }, "PhantomOrderPriceSnapshot saved")
})
