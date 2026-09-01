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
	LiquidityProviderBalanceV2,
	LiquidityProviderV4Position,
	PhantomOrderLeg,
	PhantomOrderPriceSnapshotV2,
	PhantomOrderV2,
} from "@/configs/src/types"
import { aggregatePhantomBids } from "@hyperbridge/sdk/intents-helpers"
import { blockReaders, evmRpcUrls } from "@/utils/solverBalance"
import {
	bidNonceKeyVm2,
	extractFillDataVm2,
	keccakVm2,
	orderCommitmentVm2,
	recoverBidSignerVm2,
} from "@/utils/phantom-decode"
import { UNISWAP_V4_ADDRESSES } from "@/addresses/uniswap-v4.addresses"
import { resolvePoolLeg, updateLiquidityPools, type AttributedLeg } from "@/services/liquidityPool.service"
import { readAllPages } from "@/utils/store.helpers"

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

	// The pallet bounds an order's legs (MAX_PHANTOM_ORDER_LEGS), but that cap has one home —
	// the pallet — so read to exhaustion rather than mirroring the number here.
	const legs = await readAllPages((limit, offset) =>
		PhantomOrderLeg.getByFields([["orderId", "=", commitment]], {
			limit,
			offset,
			orderBy: "legIndex",
			orderDirection: "ASC",
		}),
	)
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
	const rpcUrls = evmRpcUrls()
	const gatewayAddress = INTENT_GATEWAY_V3_ADDRESSES[phantom.chain as keyof typeof INTENT_GATEWAY_V3_ADDRESSES]
	if (!rpcUrls[phantom.chain] || !gatewayAddress) return

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
			evmRpcUrls: rpcUrls,
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
			// Lets a bid's declared V4 positions count towards the leg they back; the amounts and the
			// ownership check are read on-chain, so the bid only points at what to look at.
			uniswapV4: UNISWAP_V4_ADDRESSES,
			keccak: keccakVm2,
			getBalance: blockReaders(`${host}-${blockNumber}`).getBalance,
			logger,
		})
	} catch (err) {
		// aggregatePhantomBids already retried the whole run; reaching here means an input stayed
		// unreadable, so there is no honest snapshot to write. Skipping leaves each chain row on its
		// previous rate with a stale lastUpdatedBlock — visibly old, rather than confidently wrong.
		const msg = err instanceof Error ? `${err.name}: ${err.message}` : String(err)
		logger.error({ err, commitment, blockNumber }, `Phantom bid aggregation failed, skipping window: ${msg}`)
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
		// One row per provider per (chain, token) per block so liquidity history is preserved.
		// Every order closing on this block sweeps the same point-in-time balances, so the row is
		// written by whichever of them runs first; a solver first seen by a later order on the
		// block still gets its rows here.
		const id = `${lp.chain}-${lp.tokenAddress}-${blockNumber}-${lp.solver}`
		const existing = await LiquidityProviderBalanceV2.get(id)
		if (existing) {
			// Only the chain whose own bid declared the positions can value them, and that order is
			// not necessarily the one that wrote this row — every chain's sweep covers every chain,
			// but sees V4 positions on its own chain alone. Take the larger reading so the stored
			// value does not depend on which of the block's orders happened to run first.
			if (lp.balance > existing.balance) {
				existing.balance = lp.balance
				await existing.save()
			}
			continue
		}
		await LiquidityProviderBalanceV2.create({
			id,
			providerId: lp.solver,
			chain: lp.chain,
			blockNumber,
			tokenAddress: lp.tokenAddress,
			balance: lp.balance,
			snapshotTime,
		}).save()
	}

	// A bid is the only place a Uniswap V4 position is ever named, so this window's declarations are
	// the whole truth for the solvers that bid: their rows are replaced wholesale. A solver that did
	// not bid keeps whatever it last declared — silence is not a withdrawal — and one that bid
	// without declaring anything loses its rows, exactly as this window's weights already treat it.
	const declaredBySolver = new Map<string, Set<bigint>>()
	for (const position of aggregate.positions) {
		const declared = declaredBySolver.get(position.solver) ?? new Set<bigint>()
		declared.add(position.tokenId)
		declaredBySolver.set(position.solver, declared)
	}
	// Every verified bid sweeps balances, so lpBalances names the bidders — except one holding
	// nothing anywhere, whose only trace is the position it declared.
	const bidders = new Set([...aggregate.lpBalances.map((lp) => lp.solver), ...declaredBySolver.keys()])
	for (const solver of bidders) {
		const declared = declaredBySolver.get(solver) ?? new Set<bigint>()
		if (declared.size > 0 && !(await LiquidityProvider.get(solver))) {
			await LiquidityProvider.create({ id: solver }).save()
		}
		// Positions per solver per chain are bounded by what it declares, which nothing caps.
		const existing = await readAllPages((limit, offset) =>
			LiquidityProviderV4Position.getByFields(
				[
					["providerId", "=", solver],
					["chain", "=", phantom.chain],
				],
				{ limit, offset, orderBy: "id", orderDirection: "ASC" },
			),
		)
		for (const row of existing) {
			if (!declared.has(row.tokenId)) await LiquidityProviderV4Position.remove(row.id)
		}
		for (const tokenId of declared) {
			// Keyed by position, not by solver, so one that changed hands moves to its new owner
			// rather than being recorded under both.
			await LiquidityProviderV4Position.create({
				id: `${phantom.chain}-${tokenId}`,
				providerId: solver,
				chain: phantom.chain,
				tokenId,
				lastDeclaredBlock: blockNumber,
				lastDeclaredAt: snapshotTime,
			}).save()
		}
	}

	// Each registered leg converted to 20-byte addresses and resolved against the registry ONCE;
	// both the snapshot writes and the pool upsert consume this.
	const registeredByIndex = new Map<number, AttributedLeg>(
		legs.map((row) => {
			const leg = {
				legIndex: row.legIndex,
				tokenA: bytes32ToBytes20(row.tokenA),
				tokenB: bytes32ToBytes20(row.tokenB),
				standardAmount: row.standardAmount,
			}
			return [row.legIndex, { ...leg, resolved: resolvePoolLeg(phantom.chain, leg) }]
		}),
	)

	for (const leg of aggregate.legs) {
		const registered = registeredByIndex.get(leg.legIndex)
		// A priced leg with no registered leg means the order and the event disagree on its shape,
		// which would attach a price to the wrong tokens.
		if (!registered) {
			logger.warn({ commitment, legIndex: leg.legIndex }, "Priced leg has no registered leg, skipping")
			continue
		}
		if (!registered.resolved) {
			logger.debug({ commitment, legIndex: leg.legIndex }, "Leg tokens not registry-tracked, no pool attribution")
		}

		await PhantomOrderPriceSnapshotV2.create({
			id: `${commitment}-${blockNumber}-${leg.legIndex}`,
			commitment,
			legId: `${commitment}-${leg.legIndex}`,
			chain: phantom.chain,
			poolId: registered.resolved?.poolId,
			direction: registered.resolved?.direction,
			tokenA: registered.tokenA,
			tokenB: registered.tokenB,
			// Denormalized from the leg so a rate (medianPrice / standardAmount) is computable
			// from a single snapshot row without joining back to it.
			standardAmount: registered.standardAmount,
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
		legs: [...registeredByIndex.values()],
		priced: aggregate.legs,
	})

	logger.info({ commitment, blockNumber, pricedLegs: aggregate.legs.length }, "PhantomOrderPriceSnapshotV2 saved")
})
