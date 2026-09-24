import type { FillResult, FillerStrategy } from "@/strategies/base"
import {
	type Order,
	type ExecutionResult,
	type HexString,
	bytes32ToBytes20,
	type ERC7821Call,
	type TokenInfo,
	type IntentsCoprocessor,
	ADDRESS_ZERO,
} from "@hyperbridge/sdk"
import type { ChainClientManager, ContractInteractionService } from "@/services"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { BidPlan } from "@/services/CacheService"
import { formatUnits } from "viem"
import { type Logger , moduleLogger} from "@/services/Logger"
import type { ConfirmationPolicy } from "@/config/interpolated-curve"
import { type AssetRegistry, normalizeSymbol, } from "@/config/asset-registry"
import { Decimal } from "decimal.js"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { FundingVenue } from "@/funding/types"
import type { Signer } from "@/services/wallet"
import { paymasterReserveForToken } from "@/services/paymaster"
import type { LimitOrderStore } from "@/data/types"
import { inputFor, toRaw, toScaled } from "@/orderbook/amounts"
import { type IncomingOrder, matchLimitOrders, type LimitOrderMatch, whyUnmatched } from "@/orderbook/matching"
import { limitOrderUsdEdges, usdFactorsFrom, usdValueOf } from "@/orderbook/usd"

/**
 * One market the engine will quote, as a pair of registry symbols.
 *
 * A pair declares that the market exists and nothing more: what the filler pays
 * on it comes from the operator's limit orders. `token0 == token1` is the
 * same-asset cross-chain market, where the spread is realized in kind.
 */
export interface TradingPair {
	token0: string
	token1: string
}

/**
 * Decimal rescale that truncates when precision is lost. The SDK's
 * `adjustDecimals` rounds UP — correct for fees, where under-charging is the
 * failure mode — but profit accounting must round against itself: a credit
 * rounded up can turn an exactly break-even fill into "+1 unit profitable".
 * Callers pass non-negative credits (spreads are gated per leg before any
 * negative value could matter), where truncation equals flooring.
 */
function adjustDecimalsFloor(amount: bigint, fromDecimals: number, toDecimals: number): bigint {
	if (fromDecimals === toDecimals) return amount
	if (fromDecimals < toDecimals) return amount * BigInt(10 ** (toDecimals - fromDecimals))
	return amount / BigInt(10 ** (fromDecimals - toDecimals))
}

/** A leg matched to a configured pair, with everything needed to price it. */
interface ResolvedLeg {
	pair: TradingPair
	/** True when the leg's input is the pair's token0 (filler sells token1). */
	inputIsToken0: boolean
	/** token1 address on the chain where the exotic side of this leg settles. */
	token1Address: string
	/** Chain (state machine id) where the token1 side of this leg lives. */
	token1Chain: string
}

/** Rate context resolved for a leg: pricing rate plus the opposite side for margin telemetry. */
interface LegRates {
	/** token1 per token0 used to price this leg's output. */
	rate: Decimal
	/** The opposite side's rate (bid for ask-legs, ask for bid-legs), when available. */
	oppositeRate: Decimal | null
}

/**
 * Strategy for swaps across a configurable set of trading pairs, each priced
 * and sized by its own bid/ask curves. Supports both same-chain and cross-chain
 * orders.
 *
 * Pairs are declared as `token0`/`token1` registry symbols — e.g. USDC/CNGN,
 * USDT/CNGN, ZARP/CNGN — and any number of pairs can run in one engine. Curves
 * are quoted in **token1 per token0**; nothing assumes the quote side is a USD
 * stablecoin and no external price feed is consulted. Trade sizing is
 * pair-local in token0 units (the per-order `maxOrderSize` cap and the curve
 * amount axis); only confirmation sizing converts to USD, using the curves
 * themselves as the price feed (see `usdFactors`).
 *
 * For each (input, output) leg the engine finds the configured pair matching
 * the leg's direction:
 *  - input = token0, output = token1 → the filler *sells* token1 at the ask.
 *  - input = token1, output = token0 → the filler *buys* token1 at the bid.
 *
 * The filler holds inventory on both sides of its pairs. Profitability
 * evaluation caps each pair's legs at the pair's `maxOrderSize`, prices each
 * leg with its pair's curve (or venue), and bounds outputs by the filler's
 * real balances plus funding-venue withdrawals. Because the IntentGateway
 * releases inputs proportionally to the fraction of outputs provided, partial
 * fills (and overfills) need no extra on-chain logic.
 */
export class FXFiller implements FillerStrategy {
	name = "FXFiller"
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	/** Trading pairs served by this engine; each with its own bid/ask policies and cap. */
	private pairs: TradingPair[]
	/** Symbol → per-chain address resolution (built-ins + curated + user `[assets]`). */
	private registry: AssetRegistry
	private signer: Signer
	private logger: Logger
	/** Consecutive orders where overfill clamp activated. */
	private consecutiveClamps = 0
	/** Once set, the filler refuses all orders until restart — systemic pricing error suspected. */
	private halted = false
	/** Ceiling bps above user-requested output. Sourced from filler config. */
	private readonly maxOverfillBps: bigint
	/** Consecutive clamped evaluations before halting. Sourced from filler config. */
	private readonly maxConsecutiveClamps: number
	confirmationPolicy?: { getConfirmationBlocks: (chainId: number, amountUsd: number) => number }
	private fundingVenues: FundingVenue[]
	/** The operator's resting orders: the only thing that prices a fill. */
	private limitOrders?: LimitOrderStore

	/**
	 * @param signer          Filler's signing account for UserOp signatures.
	 * @param configService   Network/config provider for addresses and decimals.
	 * @param clientManager   Used to get viem PublicClients for chains.
	 * @param contractService Shared contract interaction service.
	 * @param pairs           Trading pairs with their bid/ask price policies and per-order caps.
	 * @param registry        Asset symbol registry resolving pair symbols per chain.
	 * @param options.confirmationPolicy Optional per-chain confirmation policy for cross-chain orders.
	 * @param options.fundingVenues  Optional funding venues for on-chain liquidity sourcing.
	 */
	constructor(
		signer: Signer,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		pairs: TradingPair[],
		registry: AssetRegistry,
		options?: {
			confirmationPolicy?: ConfirmationPolicy
			fundingVenues?: FundingVenue[]
			/** Where prices come from. Without it the engine matches nothing and fills nothing. */
			limitOrders?: LimitOrderStore
		},
	) {
		this.logger = moduleLogger(configService.loggers, "fx-simplex")
		const { confirmationPolicy, fundingVenues = [], limitOrders } = options ?? {}
		this.limitOrders = limitOrders

		if (pairs.length === 0) {
			throw new Error("FXFiller requires at least one trading pair")
		}
		const seenPairs = new Set<string>()
		for (const pair of pairs) {
			FXFiller.assertPairValid(pair, seenPairs)
		}

		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.pairs = pairs
		this.registry = registry
		this.fundingVenues = fundingVenues

		this.signer = signer
		this.maxOverfillBps = configService.getMaxOverfillBps()
		this.maxConsecutiveClamps = configService.getMaxConsecutiveClamps()
		if (confirmationPolicy) {
			this.confirmationPolicy = {
				getConfirmationBlocks: (chainId: number, amountUsd: number) =>
					confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
			}
		}
	}

	/** Per-pair invariants, shared by the constructor and addPair. Adds the accepted pair's label to `seenPairs`. */
	private static assertPairValid(pair: TradingPair, seenPairs: Set<string>): void {
		const label = `${normalizeSymbol(pair.token0)}/${normalizeSymbol(pair.token1)}`
		const reversed = `${normalizeSymbol(pair.token1)}/${normalizeSymbol(pair.token0)}`
		if (seenPairs.has(label) || seenPairs.has(reversed)) {
			throw new Error(
				`FXFiller pair ${pair.token0}/${pair.token1}: duplicate market (a pair and its reverse are the same market)`,
			)
		}
		seenPairs.add(label)
	}

	/**
	 * Adds a market to the running engine. All-or-nothing: the pair passes the
	 * same duplicate and reverse-orientation checks the constructor enforces
	 * before it is pushed.
	 */
	addPair(pair: TradingPair): void {
		const seenPairs = new Set(this.pairs.map((p) => `${normalizeSymbol(p.token0)}/${normalizeSymbol(p.token1)}`))
		FXFiller.assertPairValid(pair, seenPairs)
		this.pairs.push(pair)
	}

	/**
	 * Removes a market from the running engine. All-or-nothing: at least one
	 * market must remain and no other pair may lose its USD anchor (removing a
	 * reference feed that anchors a dependent market is rejected). In-flight
	 * rechecks of orders on the removed pair simply stop matching.
	 */
	removePair(pair: TradingPair): void {
		const index = this.pairs.indexOf(pair)
		if (index < 0) {
			throw new Error(`FXFiller pair ${pair.token0}/${pair.token1}: not a live market`)
		}
		const remaining = this.pairs.filter((_, i) => i !== index)
		if (remaining.length === 0) {
			throw new Error("FXFiller requires at least one trading pair — the last market cannot be removed live")
		}
		this.pairs.splice(index, 1)
	}

	// =========================================================================
	// Lifecycle
	// =========================================================================

	/**
	 * Call once at startup after construction.
	 * Hydrates all funding venue state before any fill sources from it.
	 */
	async initialise(): Promise<void> {
		const solver = this.signer.address as HexString
		await Promise.all(this.fundingVenues.map((v) => v.initialise(solver)))
	}

	async canFill(order: Order): Promise<boolean> {
		if (this.halted) {
			this.logger.warn({ orderId: order.id }, "FXFiller halted — rejecting order")
			return false
		}
		try {
			if (order.inputs.length !== order.output.assets.length) {
				this.logger.info(
					{ orderId: order.id, inputs: order.inputs.length, outputs: order.output.assets.length },
					"Order input/output length mismatch or empty",
				)
				return false
			}

			if ((await this.matchOrder(order)).length === 0) {
				this.logger.info(
					{
						orderId: order.id,
						sourceChain: order.source,
						destChain: order.destination,
						reason: await this.explainUnmatched(order),
					},
					"No limit order matches this order",
				)
				return false
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error in canFill")
			return false
		}
	}

	/**
	 * Evaluates whether an order is profitable to fill under the per-pair
	 * `maxOrderSize` caps (where set) and the filler's current token balances.
	 *
	 * High-level flow:
	 * - Resolve each (input, output) leg to a configured pair and direction.
	 * - Estimate each pair's total token0 notional in the order and cap it at
	 *   the pair's `maxOrderSize` when one is set; pair curves are evaluated at
	 *   that (possibly uncapped) notional.
	 * - Walk the legs, allocating from each pair's capped token0 budget and
	 *   pricing outputs at the pair's rate.
	 * - Further cap each leg by the filler's current token balance plus
	 *   funding-venue withdrawals.
	 * - Cache the resulting outputs for later use in `executeOrder`.
	 *
	 * Note: we may intentionally overfill relative to the user's requested
	 * outputs if the pair pricing makes that attractive. This is how we stay competitive.
	 */
	async calculateProfitability(order: Order): Promise<number> {
		// Cleared up front: the caller exempts partial fills from its profit floor,
		// so a stale `true` from an earlier evaluation would let a refusal through.
		if (order.id) this.contractService.cacheService.clearPartialFill(order.id)
		if (this.halted) {
			this.logger.warn({ orderId: order.id }, "FXFiller halted — rejecting order")
			return 0
		}
		try {
			const sourceChain = order.source
			const destChain = order.destination
			const { decimals: feeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(sourceChain)

			const destClient = this.clientManager.getPublicClient(destChain)
			const walletAddress = this.signer.address as HexString
			const balanceCache = new Map<string, bigint>()

			// Every leg is priced on its own: each is an input escrowed against an output,
			// and the gateway fills and credits them separately (`_partialFills` per leg).
			// A bid quotes the one leg its limit order serves and zero on every other,
			// which the gateway reads as skipping them. So a solver whose orders serve
			// some legs of an order bids on those, and others can fill the rest.
			const multiLeg = order.inputs.length > 1
			const plans: BidPlan[] = []
			// What this evaluation has already planned against each limit order, at 1e18.
			// Legs taking the same input can match the same limit order; each bid holds its
			// payout when it goes out, so a later leg sized from the order's whole
			// availability would find no room and be dropped. Sized from what is left, it
			// bids a smaller partial instead.
			const plannedOn = new Map<string, bigint>()
			for (let leg = 0; leg < order.inputs.length; leg++) {
				const input = order.inputs[leg]
				const output = order.output.assets[leg]

				// A zero requested output is degenerate on the fill path: the gateway
				// releases no escrow for it (remaining == 0), yet it would still be sized
				// and could feed the profit gate. Never bid on such a leg.
				if (!input || !output || output.amount === 0n) {
					this.logger.info({ orderId: order.id, leg }, "Skipping a leg: no priceable input/output pair")
					continue
				}

				const matches = await this.matchLeg(order, leg)
				if (matches.length === 0) {
					this.logger.info({ orderId: order.id, leg }, "Skipping a leg: no limit order matches it")
					continue
				}

				const outputToken = bytes32ToBytes20(output.token) as HexString
				const outputDecimals = await this.contractService.getTokenDecimals(outputToken, destChain)
				const inputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(input.token) as HexString,
					sourceChain,
				)

				// One bid per limit order. Each is its own fill: the full input is priced
				// against that order alone, and the gateway clamps whatever is left
				// outstanding when the bid lands. Nothing is summed here — a combined bid
				// would pay one input to several orders at once.
				for (const candidate of matches) {
					let partialFill = false
					const fundingCalls: ERC7821Call[] = []
					// What this limit order alone will pay, in the output token's own units:
					// `min(offer, remaining − reserved)`, less whatever an earlier leg of this
					// order already planned against it, so the order never offers more than it
					// has left even when the wallet holds more.
					const left = candidate.available - (plannedOn.get(candidate.order.id) ?? 0n)
					if (left <= 0n) {
						this.logger.info(
							{ orderId: order.id, leg, limitOrder: candidate.order.id },
							"Skipping a bid: an earlier leg of this order already planned everything the limit order has left",
						)
						continue
					}
					const offered = toRaw(candidate.offer < left ? candidate.offer : left, outputDecimals)
					if (offered === 0n) continue

					// The bid is the limit order's own rate: its whole offer for the input,
					// capped by what it has left. The gateway credits the swapper the ask and
					// pays the swapper and the protocol whatever is above it, so a better price
					// reaches the swapper, and bids from several operators rank by price rather
					// than all tying at the ask.
					const targetOutput = offered

					// Whether this order may be filled below what the user asked for. Both
					// chains allow it: `ExtrinsicIntents._fillCrossChain` keeps cumulative
					// progress in `_partialFills[commitment][outputToken]`, clears
					// `_filled[commitment]` on an under-fill so another solver can take the
					// rest, and releases escrow proportionally through `RedeemEscrowPartial`.
					//
					//  - An order carrying output calldata reverts (`PartialFillNotAllowed`):
					//    the attached call runs only on a full fill, so the gateway will not
					//    release escrow without it. That holds on both paths.
					//  - An order already partially filled has had its escrow drawn down,
					//    while the P&L below reads the leg's escrowed input as if it were
					//    intact. Refuse rather than mis-price it.
					const partialEligibleCheap = (order.output.call ?? "0x").length <= 2
					// The prior-partial probe is a contract read, and most orders fill fully
					// and never consult it — so it runs only once an under-fill is actually on
					// the table, and at most once per evaluation.
					let priorPartialChecked: boolean | undefined
					const partialEligible = async (): Promise<boolean> => {
						if (!partialEligibleCheap) return false
						if (priorPartialChecked === undefined) {
							priorPartialChecked = !(await this.hasExistingPartialFill(order, destChain))
						}
						return priorPartialChecked
					}


					let deadlineTimestamp: bigint | undefined
					try {
						const latestBlock = await destClient.getBlock()
						const blockTimeMs = destClient.chain?.blockTime
						const blockTimeSec = blockTimeMs ? blockTimeMs / 1000 : 2
						const remainingBlocks = order.deadline > latestBlock.number ? Number(order.deadline - latestBlock.number) : 0
						deadlineTimestamp = BigInt(Math.floor(Number(latestBlock.timestamp) + remainingBlocks * blockTimeSec))
					} catch (err) {
						this.logger.warn({ err, destChain }, "Failed to estimate deadline timestamp, using fallback")
					}

					// A shortfall is either the limit order running out or its price landing
					// under what the swapper asked for. Cross-chain neither can be filled;
					// same-chain both can, as far as the payout goes.
					if (targetOutput < output.amount && !(await partialEligible())) {
						this.logger.info(
							{
								orderId: order.id,
								limitOrder: candidate.order.id,
								available: formatUnits(
									candidate.available,
									18,
								),
								userRequested: output.amount.toString(),
								payout: targetOutput.toString(),
								crossChain: sourceChain !== destChain,
							},
							"Skipping order: the matched limit order cannot cover it and this order cannot be partially filled",
						)
						continue
					}
					if (targetOutput < output.amount) partialFill = true

					// Nothing is ever bid above the ask now, so the ceiling cannot be crossed
					// on the way out. It still says something worth hearing: an offer far past
					// what the swapper wanted is a limit order priced well away from the
					// market, which is usually a mistake in the operator's terms.
					const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n
					if (offered > overfillCeiling) {
						this.logger.warn(
							{
								orderId: order.id,
								limitOrder: candidate.order.id,
								token: output.token,
								userRequested: output.amount.toString(),
								offered: offered.toString(),
								ceiling: overfillCeiling.toString(),
								maxOverfillBps: this.maxOverfillBps.toString(),
							},
							"Limit order offers far more than the swapper asked for",
						)
					}

					// Spend the free wallet balance first, down to the reserve — the paymaster's
					// gas pull during validatePaymasterUserOp plus the vault's configured
					// minBalance — then source any remaining shortfall from the funding venues.
					const tokenAddress = outputToken.toLowerCase()
					const balance = await this.getAndCacheBalance(tokenAddress, walletAddress, destClient, balanceCache)

					let reserve = paymasterReserveForToken(destChain, tokenAddress, this.configService)
					for (const venue of this.fundingVenues) {
						reserve += venue.walletReserveForToken(destChain, tokenAddress)
					}
					const usableWallet = balance > reserve ? balance - reserve : 0n

					const walletContribution = targetOutput < usableWallet ? targetOutput : usableWallet

					let credited = 0n
					let needed = targetOutput - walletContribution
					for (const venue of this.fundingVenues) {
						if (needed <= 0n) break
						const planned = await venue.planWithdrawalForToken(destChain, walletAddress, tokenAddress, needed, deadlineTimestamp)
						if (planned.calls.length > 0) {
							fundingCalls.push(...planned.calls)
							credited += planned.credited
							needed -= planned.credited
						}
					}

					const effectiveBalance = walletContribution + credited
					const finalOutputAmount = effectiveBalance > targetOutput ? targetOutput : effectiveBalance

					if (finalOutputAmount === 0n) {
						this.logger.info(
							{
								orderId: order.id,
								limitOrder: candidate.order.id,
								token: output.token,
								inputAmount: input.amount.toString(),
								fillerBalance: balance.toString(),
							},
							"Skipping order: no available balance for the output token",
						)
						continue
					}

					// Any shortfall makes this an under-fill, whatever caused it — the limit
					// order running out, or not holding enough of the output token. The
					// gateway does not care which: an under-fill on a cross-chain order or
					// one carrying output calldata reverts, so both clear the same check.
					if (finalOutputAmount < output.amount) {
						if (!(await partialEligible())) {
							this.logger.info(
								{
									orderId: order.id,
									limitOrder: candidate.order.id,
									token: output.token,
									available: finalOutputAmount.toString(),
									userRequested: output.amount.toString(),
									crossChain: sourceChain !== destChain,
									hasCalldata: (order.output.call ?? "0x").length > 2,
								},
								"Skipping order: cannot fill it in full and it cannot be partially filled",
							)
							continue
						}
						partialFill = true
					}

					// A bid on one leg of a multi-leg order leaves the other legs open, so it
					// is a partial fill whatever it pays on its own leg, and needs an order
					// that may be filled in parts.
					if (multiLeg && !partialFill) {
						if (!(await partialEligible())) {
							this.logger.info(
								{ orderId: order.id, leg, limitOrder: candidate.order.id },
								"Skipping a bid: a bid on one leg of a multi-leg order is a partial fill, which this order does not allow",
							)
							continue
						}
						partialFill = true
					}

					// Decrement the wallet pool by what this fill drew from it (vault-sourced
					// tokens are tracked by the venue's own reservations).
					const walletRemaining = balance - walletContribution
					balanceCache.set(tokenAddress, walletRemaining > 0n ? walletRemaining : 0n)

					// The venue clamp is gone with the curves, so a fill can never be clamped —
					// the halt subsystem is left in place but dormant (always recorded as a
					// clean, unclamped outcome).
					this.recordOrderOutcome(false, order.id)

					// The take signed beside the output. `fillOrder` settles the output against
					// it as this bid's rate, credits the swapper `take * ask / escrow`, and
					// charges the solver the escrow it releases at that rate.
					//
					//  - A payout that covers the ask takes the whole input, so the order can be
					//    filled in one go. At the full offer that is exactly the limit order's
					//    rate; a payout cut short by what is left sits between it and the ask.
					//  - A payout below the ask is a partial fill at the limit order's rate: the
					//    input that payout buys at its price. It is capped at the most the
					//    gateway accepts for it (`RateBelowOrder` refuses a take whose credit
					//    at the order's rate exceeds the output), which only binds when the
					//    limit order's price is the ask itself.
					const releasedInput =
						finalOutputAmount >= output.amount
							? input.amount
							: minBigInt(
									toRaw(
										inputFor({
											side: candidate.order.side,
											outputAmount: toScaled(finalOutputAmount, outputDecimals),
											price: BigInt(candidate.order.price),
											inputDecimals,
										}),
										inputDecimals,
									),
									(finalOutputAmount * input.amount) / output.amount,
									input.amount,
								)
					if (releasedInput === 0n) {
						this.logger.info(
							{
								orderId: order.id,
								limitOrder: candidate.order.id,
								payout: finalOutputAmount.toString(),
								userRequested: output.amount.toString(),
							},
							"Skipping a bid: its payout is too small to release any escrow",
						)
						continue
					}

					// One entry per leg, zero on every leg but this one: the gateway skips those.
					const fillerOutputs: TokenInfo[] = order.output.assets.map((asset, i) => ({
						token: asset.token,
						amount: i === leg ? finalOutputAmount : 0n,
					}))
					const fillerInputs: TokenInfo[] = order.inputs.map((asset, i) => ({
						token: asset.token,
						amount: i === leg ? releasedInput : 0n,
					}))

					const usdFactors = usdFactorsFrom(limitOrderUsdEdges(await this.limitOrders!.open()))
					const outputSymbol = this.registry.symbolFor(outputToken, destChain)

					// A same-asset market realizes its spread in kind: escrow released minus
					// output paid, in the asset's own units. Positive iff the filler nets the
					// asset — a sign check valid for any asset, since it never crosses units.
					const sameAsset =
						outputSymbol !== null &&
						normalizeSymbol(outputSymbol) === normalizeSymbol(candidate.order.base) &&
						normalizeSymbol(outputSymbol) === normalizeSymbol(candidate.order.quote)
					let realizedSpreadProfit = 0n
					let sameAssetEdgeUsd = new Decimal(0)
					let sameAssetProfitable = true
					if (sameAsset) {
						const convertedInput = adjustDecimalsFloor(releasedInput, inputDecimals, outputDecimals)
						const spread = convertedInput - finalOutputAmount
						if (spread <= 0n) sameAssetProfitable = false
						realizedSpreadProfit = adjustDecimalsFloor(spread, outputDecimals, feeTokenDecimals)
						const spreadUsd = outputSymbol && usdValueOf(usdFactors, outputSymbol, new Decimal(formatUnits(spread, outputDecimals)))
						if (spreadUsd) sameAssetEdgeUsd = spreadUsd
					}

					// Bidding the limit order's own rate hands anything above the ask to the
					// swapper and the protocol, so none of it is margin this bid keeps.
					const payoutSurplusUsd = new Decimal(0)

					const { totalCostInSourceFeeToken, relayerFeeInSourceFeeToken, dispatchFee } =
						await this.contractService.estimateGasFillPost(order)

					// `fillOrder` dispatches the escrow-release message back to the source
					// chain, and HyperApp.dispatchWithFeeToken pulls `dispatchFee` from this
					// same wallet in the destination host's fee token — which on most chains
					// is the USDC the fill is already paying out. The sizing above committed
					// the balance to outputs without knowing this figure (it is only priced
					// here, after the funding calls it depends on exist), so the affordability
					// check has to happen now. A cross-chain order cannot be partially filled,
					// so shrinking the fill is not on the table: either the residue covers the
					// dispatch or the order is not ours to take.
					if (sourceChain !== destChain && dispatchFee > 0n) {
						const feeToken = await this.contractService.getFeeTokenWithDecimals(destChain)
						const feeTokenLower = feeToken.address.toLowerCase()
						// `balanceCache` holds the output token's balance net of what the fill
						// draws from it; a fee token the fill did not pay out is read fresh.
						const residual = await this.getAndCacheBalance(feeTokenLower, walletAddress, destClient, balanceCache)
						const required = dispatchFee + paymasterReserveForToken(destChain, feeTokenLower, this.configService)
						if (residual < required) {
							this.logger.info(
								{
									orderId: order.id,
									feeToken: feeTokenLower,
									residual: formatUnits(residual, feeToken.decimals),
									dispatchFee: formatUnits(dispatchFee, feeToken.decimals),
									required: formatUnits(required, feeToken.decimals),
								},
								"Skipping order: fill leaves too little of the fee token to dispatch the escrow release",
							)
							continue
						}
					}

					// GATE 1 — execution cost (independent). order.fees exist solely to pay
					// for execution: the fill gas plus, for cross-chain orders, the relayer
					// fee for delivering the escrow-release message back to the source chain
					// (RELAYER_MESSAGE_GAS priced on the source chain; 0 for same-chain). The
					// swap spread is NOT credited here — fees must cover cost on their own.
					const executionCost = totalCostInSourceFeeToken + relayerFeeInSourceFeeToken
					const partialEdgeUsd = sameAssetEdgeUsd.plus(payoutSurplusUsd)

					// GATE 1 — full fills only. A partial collects NO `order.fees`: the gateway
					// releases those to whoever completes the order (`_withdraw(..., finalize)`
					// with `finalize = isFullyFilled`), so there is no fee revenue to test. What
					// a partial earns instead is its spread net of gas, which is exactly what
					// `totalProfit` reports below — and the caller already refuses anything that
					// does not score above zero.
					if (!partialFill && order.fees < executionCost) {
						this.logger.info(
							{
								orderId: order.id,
								orderFees: formatUnits(order.fees, feeTokenDecimals),
								fillGas: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
								relayerFee: formatUnits(relayerFeeInSourceFeeToken, feeTokenDecimals),
								executionCost: formatUnits(executionCost, feeTokenDecimals),
							},
							"Skipping order: attached fees do not cover execution cost (fill gas + relayer fee)",
						)
						continue
					}

					// GATE 2 — same-asset spread (independent). A same-asset fill must net the
					// filler the asset. Cross-asset fills are not gated: they fill at the rate
					// the operator's own limit order signed for, which is the price they
					// declared acceptable.
					if (sameAsset && !sameAssetProfitable) {
						this.logger.info(
							{ orderId: order.id, realizedSpreadProfit: formatUnits(realizedSpreadProfit, feeTokenDecimals) },
							"Skipping order: a same-asset fill does not net a positive spread",
						)
						continue
					}

					const feeProfit = order.fees - executionCost
					// Both gates passed → the order is profitable. This number is only the
					// ranking / >0 execute signal, never a funds gate (the two gates above
					// already decided).
					//
					// Full fill: fee surplus (USD) plus the realized same-asset spread — for a
					// non-USD same-asset market the spread term is in that asset's units, so
					// the magnitude is a rough signal rather than a true dollar figure; its
					// sign is always correct.
					//
					// Partial fill: the USD edge, gross. Gas is deliberately not netted off —
					// a partial collects no fees, so netting gas would put every one of them
					// at or below zero and nothing would ever fill. What pays for the gas is
					// the margin in the operator's own limit order, which the engine cannot
					// measure; the caller exempts partials from the profit floor for the same
					// reason.
					//
					// A cross-chain partial pays one cost a same-chain one does not: the
					// relayer fee carrying `RedeemEscrowPartial` back to the source. It is not
					// netted here either, for the same reason and with the same consequence,
					// so the operator's margin has to cover the message as well as the gas.
					const totalProfit = partialFill
						? partialEdgeUsd.toNumber()
						: Number.parseFloat(formatUnits(feeProfit + realizedSpreadProfit, feeTokenDecimals))

					this.logger.info(
						{
							orderId: order.id,
							sourceChain,
							destChain,
							crossChain: sourceChain !== destChain,
							leg,
							limitOrder: candidate.order.id,
							book: candidate.order.book,
							side: candidate.order.side,
							price: candidate.order.price,
							offer: candidate.offer.toString(),
							limitOrders: matches.length,
							available: candidate.available.toString(),
							payout: targetOutput.toString(),
							orderFees: formatUnits(order.fees, feeTokenDecimals),
							fillGas: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
							relayerFee: formatUnits(relayerFeeInSourceFeeToken, feeTokenDecimals),
							executionCost: formatUnits(executionCost, feeTokenDecimals),
							feeProfit: formatUnits(feeProfit, feeTokenDecimals),
							realizedSpreadProfit: formatUnits(realizedSpreadProfit, feeTokenDecimals),
							payoutSurplusUsd: payoutSurplusUsd.toString(),
							totalProfit,
							profitable: totalProfit > 0,
						},
						"FX swap profitability evaluation",
					)

					plannedOn.set(
						candidate.order.id,
						(plannedOn.get(candidate.order.id) ?? 0n) + toScaled(finalOutputAmount, outputDecimals),
					)
					plans.push({
						limitOrderId: candidate.order.id,
						leg,
						payout: toScaled(finalOutputAmount, outputDecimals),
						fillerOutputs,
						fillerInputs,
						fundingCalls: [...fundingCalls],
						partialFill,
						profit: totalProfit,
					})

					// An order carrying output calldata takes exactly one bid. The attached
					// call runs only on a full fill, so the gateway answers anything less with
					// `PartialFillNotAllowed`: a second bid could never add to the first, and
					// would only burn gas reverting once the first one landed.
					if (!partialEligibleCheap) break
				}
			}

			if (plans.length === 0) return 0
			if (order.id) {
				this.contractService.cacheService.setBidPlans(order.id, plans)
				// The first bid is what the single-bid path still reads when it asks what
				// this order is being filled with.
				this.contractService.cacheService.setFillerOutputs(order.id, plans[0].fillerOutputs, plans[0].fillerInputs)
				this.contractService.cacheService.setPartialFill(order.id, plans[0].partialFill)
				this.contractService.cacheService.setMatchedLimitOrder(order.id, [
					{ limitOrderId: plans[0].limitOrderId, payout: plans[0].payout },
				])
				if (plans[0].fundingCalls.length > 0) {
					this.contractService.cacheService.setFundingPrepends(order.id, plans[0].fundingCalls)
				} else {
					this.contractService.cacheService.clearFundingPrepends(order.id)
				}
			}

			// What the strategy is worth on this order is what all of its bids earn.
			return plans.reduce((total, plan) => total + plan.profit, 0)
		} catch (error) {
			this.logger.error({ err: error }, "Error calculating profitability")
			return 0
		}
	}

	/**
	 * Executes an order by submitting a bid via the IntentsCoprocessor.
	 *
	 * Assumes that `calculateProfitability` has already been called for the
	 * given order so that filler outputs are cached in `contractService`.
	 * This method only orchestrates the bid construction and submission; the
	 * actual token movements are handled on-chain by the IntentGateway.
	 */
	async executeOrder(order: Order, intentsCoprocessor?: IntentsCoprocessor): Promise<ExecutionResult> {
		const startTime = Date.now()

		try {
			if (!intentsCoprocessor) {
				return {
					success: false,
					error: "FXFiller requires the UserOp/Hyperbridge path (intentsCoprocessor must be provided)",
				}
			}

			return await this.submitBid(order, startTime, intentsCoprocessor)
		} catch (error) {
			this.logger.error({ err: error }, "Error executing FX swap order")
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	// =========================================================================
	// Private — Execution
	// =========================================================================

	/**
	 * Prepares and submits a bid UserOp to Hyperbridge for the given order.
	 *
	 * Uses the filler outputs previously cached by `calculateProfitability`.
	 * Approval bundling and UserOp construction are handled by
	 * `ContractInteractionService.prepareBidUserOp`.
	 */
	private async submitBid(
		order: Order,
		startTime: number,
		intentsCoprocessor: IntentsCoprocessor,
	): Promise<FillResult> {
		const entryPointAddress = this.configService.getEntryPointAddress(order.destination)
		if (!entryPointAddress) {
			return {
				success: false,
				error: `EntryPoint not configured for chain ${order.destination}`,
			}
		}

		const solverAccountAddress = this.signer.address as HexString

		// Prepare the signed UserOp for bid submission (bundles approvals + fillOrder internally)
		const { commitment, userOp, bid } = await this.contractService.prepareBidUserOp(
			order,
			entryPointAddress,
			solverAccountAddress,
		)

		const bidResult = await intentsCoprocessor.submitBid(commitment, userOp, bid)

		const endTime = Date.now()
		if (bidResult.success) {
			this.logger.info({ commitment }, "Bid submitted successfully")
			return {
				success: true,
				txHash: bidResult.extrinsicHash,
				strategyUsed: this.name,
				processingTimeMs: endTime - startTime,
				commitment,
				bid,
			}
		}

		this.logger.error({ commitment, error: bidResult.error, pending: bidResult.pending }, "Bid submission failed")
		// `pending` and the hash ride along: a pooled extrinsic that later lands
		// reserves a deposit, so the sweep must be able to find and trace it.
		return {
			success: false,
			pending: bidResult.pending === true,
			txHash: bidResult.extrinsicHash,
			error: bidResult.error,
			commitment,
			bid,
		}
	}

	// =========================================================================
	// Private — Helpers
	// =========================================================================

	/**
	 * Update consecutive-clamp counter after a successful order evaluation.
	 * Only venue-priced legs feed this counter (see clamp site) — a streak of those
	 * is the signal that a live market source has gone off (stale pool, manipulated
	 * venue) and the filler should stop until an operator investigates. Offline
	 * price-curve clamps warn but never reach here.
	 */
	private recordOrderOutcome(clamped: boolean, orderId: string | undefined) {
		if (clamped) {
			this.consecutiveClamps += 1
			if (this.consecutiveClamps >= this.maxConsecutiveClamps) {
				this.halted = true
				this.logger.error(
					{ orderId, consecutiveClamps: this.consecutiveClamps, maxConsecutiveClamps: this.maxConsecutiveClamps },
					"FXFiller HALTED — venue-priced overfill clamp triggered consecutively; restart required after investigation",
				)
			}
		} else {
			this.consecutiveClamps = 0
		}
	}

	public isHalted(): boolean {
		return this.halted
	}

	/** Operator acknowledgement after investigating a self-halt; resumes filling. */
	public resetHalt(): void {
		if (!this.halted) return
		this.halted = false
		this.consecutiveClamps = 0
		this.logger.warn("FXFiller halt reset by operator — resuming order evaluation")
	}

	/**
	 * Given a single (input, output) leg and the remaining token0 budget of its
	 * pair, computes how much token0 notional to allocate to this leg and the
	 * corresponding maximum output amount at the pair's rate.
	 *
	 * `rate` is **token1 per 1 token0**:
	 * - token0 input → token1 output: token0 × rate → token1 amount.
	 * - token1 input → token0 output: token1 ÷ rate → token0 amount.
	 *
	 * Returns `null` when this leg cannot consume any of the pair's remaining
	 * budget (e.g. the cap has already been exhausted).
	 */
	/**
	 * Whether any solver has already delivered output against this order.
	 *
	 * Fails closed: a read that errors returns true, so an order we cannot price
	 * confidently is left alone rather than bid on with stale escrow assumptions.
	 */
	private async hasExistingPartialFill(order: Order, chain: string): Promise<boolean> {
		try {
			const filled = await this.contractService.partialFillsFor(order, chain)
			return filled.some((amount) => amount > 0n)
		} catch (err) {
			this.logger.warn(
				{ orderId: order.id, chain, err },
				"Could not read existing partial fills; treating the order as already touched",
			)
			return true
		}
	}

	/**
	 * Reads and caches the filler's balance for a token on the destination chain.
	 *
	 * Normalizes the token address, checks an in-memory cache, and only hits
	 * the chain (native `getBalance` or ERC20 `balanceOf`) on a cache miss.
	 * This allows multiple legs within a single profitability evaluation to
	 * share the same balance pool.
	 */
	private async getAndCacheBalance(
		tokenAddressLower: string,
		walletAddress: HexString,
		// biome-ignore lint/suspicious/noExplicitAny: viem public client type varies per chain
		destClient: any,
		balanceCache: Map<string, bigint>,
	): Promise<bigint> {
		const key = tokenAddressLower.toLowerCase()
		const cached = balanceCache.get(key)
		if (cached !== undefined) {
			return cached
		}

		let balance: bigint
		if (key === ADDRESS_ZERO.toLowerCase()) {
			balance = await destClient.getBalance({ address: walletAddress })
		} else {
			balance = await destClient.readContract({
				abi: ERC20_ABI,
				address: key as HexString,
				functionName: "balanceOf",
				args: [walletAddress],
			})
		}

		balanceCache.set(key, balance)
		return balance
	}

	/**
	 * Resolves every (input, output) leg of an order to a configured pair in one
	 * pass. Returns null if any leg matches no pair (or a disabled direction).
	 *
	 * The address-level classification is cached per order id in the shared
	 * cache (as `CachedPairClassification`, one entry per leg) so repeated
	 * evaluations skip re-derivation; pair resolution itself is re-run per
	 * strategy since pair sets differ between engine instances.
	 */
	/** Whether any leg of the order matches a limit order: what `canFill` asks. */
	private async matchOrder(order: Order): Promise<LimitOrderMatch[]> {
		const matches: LimitOrderMatch[] = []
		for (let leg = 0; leg < order.inputs.length; leg++) {
			matches.push(...(await this.matchLeg(order, leg)))
		}
		return matches
	}

	/**
	 * The limit orders one leg of an order is priced against, best offer first, or
	 * none when nothing serves it.
	 *
	 * Amounts cross into the matcher at 1e18, the unit limit orders are kept in,
	 * and the payout comes back in the same unit for the caller to bring down to
	 * the output token's own decimals.
	 */
	private async matchLeg(order: Order, leg: number): Promise<LimitOrderMatch[]> {
		if (!this.limitOrders) return []
		const incoming = await this.incomingFor(order, leg)
		if (!incoming) return []
		return matchLimitOrders(await this.limitOrders.open(), incoming, (symbol, chain) =>
			this.registry.getAddress(symbol, chain),
		)
	}

	/** Why no limit order serves the order, leg by leg, for the line that says it was passed over. */
	private async explainUnmatched(order: Order): Promise<string> {
		if (!this.limitOrders) return "limit orders are not loaded"
		const open = await this.limitOrders.open()
		const reasons: string[] = []
		for (let leg = 0; leg < order.inputs.length; leg++) {
			const incoming = await this.incomingFor(order, leg)
			const reason = incoming
				? whyUnmatched(open, incoming, (symbol, chain) => this.registry.getAddress(symbol, chain))
				: `its input token is not in the asset registry on ${order.source}`
			reasons.push(order.inputs.length > 1 ? `leg ${leg}: ${reason}` : reason)
		}
		return reasons.join("; ")
	}

	/**
	 * One leg as the matcher sees it, or null when its input token is not one the asset
	 * registry knows on the source chain.
	 *
	 * Amounts cross into the matcher at 1e18, the unit limit orders are kept in.
	 */
	private async incomingFor(order: Order, leg: number): Promise<IncomingOrder | null> {
		const input = order.inputs[leg]
		const output = order.output.assets[leg]
		if (!input || !output) return null
		const inputToken = bytes32ToBytes20(input.token) as HexString
		const outputToken = bytes32ToBytes20(output.token) as HexString

		const inputSymbol = this.registry.symbolFor(inputToken, order.source)
		if (!inputSymbol) return null

		const [inputDecimals, outputDecimals] = await Promise.all([
			this.contractService.getTokenDecimals(inputToken, order.source),
			this.contractService.getTokenDecimals(outputToken, order.destination),
		])
		return {
			source: order.source,
			destination: order.destination,
			inputSymbol,
			outputToken,
			inputNet: toScaled(input.amount, inputDecimals),
			requestedOutput: toScaled(output.amount, outputDecimals),
			outputDecimals,
		}
	}

	/**
	 * The order's input in **USD**, or null when no limit order connects its input
	 * token to a dollar.
	 *
	 * The core filler feeds this to the per-chain confirmation curves, whose
	 * amount axis is dollars. Null is the honest answer when the route is missing:
	 * the caller waits on this figure, and inventing one would under-wait a large
	 * order against a source chain that has not finalised.
	 */
	async getOrderUsdValue(order: Order): Promise<{ inputUsd: Decimal } | null> {
		if (!this.limitOrders || order.inputs.length === 0) return null

		// Every leg's input counts: a multi-leg order escrows them all on the source chain.
		// One leg nothing prices makes the whole figure unknown rather than understated.
		const factors = usdFactorsFrom(limitOrderUsdEdges(await this.limitOrders.open()))
		let inputUsd = new Decimal(0)
		for (const input of order.inputs) {
			const inputToken = bytes32ToBytes20(input.token) as HexString
			const symbol = this.registry.symbolFor(inputToken, order.source)
			if (!symbol) return null
			const decimals = await this.contractService.getTokenDecimals(inputToken, order.source)
			const legUsd = usdValueOf(factors, symbol, new Decimal(formatUnits(input.amount, decimals)))
			if (!legUsd) return null
			inputUsd = inputUsd.plus(legUsd)
		}
		return inputUsd.gt(0) ? { inputUsd } : null
	}
}

function minBigInt(first: bigint, ...rest: bigint[]): bigint {
	return rest.reduce((min, value) => (value < min ? value : min), first)
}
