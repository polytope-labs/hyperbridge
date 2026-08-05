import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp, replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { ENV_CONFIG } from "@/constants"
import { INTENT_GATEWAY_V3_ADDRESSES } from "@/intent-gateway-v3-addresses"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { SOLVER_ACCOUNT_ADDRESSES } from "@/solver-account-addresses"
import {
	LiquidityProvider,
	LiquidityProviderBalance,
	PhantomOrderLeg,
	PhantomOrderPriceSnapshotV2,
	PhantomOrderV2,
} from "@/configs/src/types"
import { aggregatePhantomBids, setAggregationFetch } from "@hyperbridge/sdk/intents-helpers"
import { safeFetch } from "@/utils/safeFetch"
import { bidNonceKeyVm2, extractFillDataVm2, orderCommitmentVm2, recoverBidSignerVm2 } from "@/utils/phantom-decode"
import { resolvePoolLeg, updateLiquidityPools, type RegisteredLeg } from "@/services/liquidityPool.service"

// The aggregation's RPC helpers run inside the SubQuery VM2 sandbox, which has no global `fetch`.
// Inject the indexer's sandbox-safe HTTP client so its JSON-RPC calls work here.
setAggregationFetch(safeFetch)

// Triggered by PhantomBidWindowExhausted once a phantom order's bid window closes, so every bid is
// already in. The order prices several directed legs at once, so its bids aggregate into one
// snapshot per leg. The heavy lifting lives in aggregatePhantomBids(); this handler just resolves
// endpoints and persists the result.
export const handlePhantomOrderPrices = wrap(async (event: SubstrateEvent): Promise<void> => {
	const blockNumber = event.block.block.header.number.toBigInt()
	const blockHash = event.block.block.header.hash.toString()

	const [commitmentData] = event.event.data
	const commitment = commitmentData.toHex()

	const phantom = await PhantomOrderV2.get(commitment)
	if (!phantom) return

	// MAX_PHANTOM_ORDER_LEGS in the pallet caps an order at 128 legs (64 pairs, both directions),
	// so one read covers the order.
	const legs = await PhantomOrderLeg.getByOrderId(commitment, { limit: 128 })
	if (legs.length === 0) return

	// Every leg's snapshot is written in one pass, so any snapshot for this block means the whole
	// order was handled. Keyed by fields rather than a specific leg index because only quoted legs
	// get snapshots, and which legs were quoted is not knowable before aggregating.
	const handled = await PhantomOrderPriceSnapshotV2.getByFields(
		[
			["commitment", "=", commitment],
			["blockNumber", "=", blockNumber],
		],
		{ limit: 1 },
	)
	if (handled.length > 0) return

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

	// RPC per supported EVM chain — the aggregation sweeps every LP's liquidity across all of them,
	// not just the phantom order's destination chain.
	const evmRpcUrls: Record<string, string> = {}
	for (const [stateMachineId, url] of Object.entries(ENV_CONFIG)) {
		if (!stateMachineId.startsWith("EVM-")) continue
		const http = replaceWebsocketWithHttp(url ?? "")
		if (http) evmRpcUrls[stateMachineId] = http
	}
	const gatewayAddress = INTENT_GATEWAY_V3_ADDRESSES[phantom.chain as keyof typeof INTENT_GATEWAY_V3_ADDRESSES]
	if (!evmRpcUrls[phantom.chain] || !gatewayAddress) return

	// A bid only counts if its sender delegates to this, so with no configured address there is
	// nothing to verify against and no snapshot to write. Worth a warning rather than a silent skip:
	// the chain is otherwise fully configured, so this is a gap in config-{mainnet,testnet}.json.
	const solverAccount = SOLVER_ACCOUNT_ADDRESSES[phantom.chain]
	if (!solverAccount) {
		logger.warn(
			{ chain: phantom.chain, commitment },
			"No SolverAccount configured for chain, skipping price snapshot",
		)
		return
	}

	let aggregate
	try {
		aggregate = await aggregatePhantomBids({
			nodeUrl,
			evmRpcUrls,
			chain: phantom.chain,
			gatewayAddress,
			commitment,
			yieldVaults: YIELD_VAULT_ADDRESSES,
			solverAccount,
			// viem's keccak throws in the VM2 sandbox; inject the indexer's ethers-based equivalents.
			extractFill: extractFillDataVm2,
			recoverSigner: recoverBidSignerVm2,
			bidNonceKey: bidNonceKeyVm2,
			orderCommitment: orderCommitmentVm2,
			logger,
		})
	} catch (err) {
		const msg = err instanceof Error ? `${err.name}: ${err.message}` : String(err)
		logger.warn({ err, commitment }, `Phantom bid aggregation failed: ${msg}`)
		return
	}
	if (!aggregate) return

	const blockTimestamp = await getBlockTimestamp(blockHash, host)
	const snapshotTime = timestampToDate(blockTimestamp)

	for (const lp of aggregate.lpBalances) {
		// Group balances under a LiquidityProvider keyed by solver address so they can be read as a
		// nested array. Upsert the provider (one per solver) before linking its balances.
		if (!(await LiquidityProvider.get(lp.solver))) {
			await LiquidityProvider.create({ id: lp.solver }).save()
		}
		// One row per provider per (chain, token) per snapshot so liquidity history is preserved and
		// each balance is attributable to the snapshot whose weighted median it fed.
		await LiquidityProviderBalance.create({
			id: `${lp.chain}-${lp.tokenAddress}-${commitment}-${blockNumber}-${lp.solver}`,
			providerId: lp.solver,
			chain: lp.chain,
			commitment,
			blockNumber,
			tokenAddress: lp.tokenAddress,
			balance: lp.balance,
			snapshotTime,
		}).save()
	}

	// One map, both shapes: the stored row (for the snapshot's FK and denormalized fields) and its
	// 20-byte-address form (for registry resolution).
	const registeredByIndex = new Map(
		legs.map((leg) => [
			leg.legIndex,
			{
				row: leg,
				resolvable: {
					legIndex: leg.legIndex,
					tokenA: bytes32ToBytes20(leg.tokenA),
					tokenB: bytes32ToBytes20(leg.tokenB),
					standardAmount: leg.standardAmount,
				} satisfies RegisteredLeg,
			},
		]),
	)

	for (const leg of aggregate.legs) {
		const registered = registeredByIndex.get(leg.legIndex)
		// A priced leg with no registered leg means the order and the event disagree on its shape,
		// which would attach a price to the wrong tokens.
		if (!registered) {
			logger.warn({ commitment, legIndex: leg.legIndex }, "Priced leg has no registered leg, skipping")
			continue
		}
		const { row, resolvable } = registered

		const resolved = resolvePoolLeg(phantom.chain, resolvable)
		if (!resolved) {
			logger.debug({ commitment, legIndex: leg.legIndex }, "Leg tokens not registry-tracked, no pool attribution")
		}

		await PhantomOrderPriceSnapshotV2.create({
			id: `${commitment}-${blockNumber}-${leg.legIndex}`,
			commitment,
			legId: row.id,
			chain: phantom.chain,
			poolId: resolved?.poolId,
			direction: resolved?.direction,
			tokenA: bytes32ToBytes20(row.tokenA),
			tokenB: bytes32ToBytes20(row.tokenB),
			// Denormalized from the leg so a rate (medianPrice / standardAmount) is computable
			// from a single snapshot row without joining back to it.
			standardAmount: row.standardAmount,
			blockNumber,
			lowestPrice: leg.lowestPrice,
			highestPrice: leg.highestPrice,
			medianPrice: leg.medianPrice,
			bidCount: leg.bidCount,
			snapshotTime,
		}).save()
	}

	await updateLiquidityPools({
		chain: phantom.chain,
		blockNumber,
		snapshotTime,
		pairs: [...registeredByIndex.values()].map((registered) => registered.resolvable),
		legs: aggregate.legs,
	})

	logger.info({ commitment, blockNumber, pricedLegs: aggregate.legs.length }, "PhantomOrderPriceSnapshotV2 saved")
})
