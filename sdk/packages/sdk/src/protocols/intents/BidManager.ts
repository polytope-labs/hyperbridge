import { decodeFunctionData, concat } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { ADDRESS_ZERO, bytes32ToBytes20 } from "@/utils"
import type {
	Order,
	HexString,
	PackedUserOperation,
	SubmitBidOptions,
	FillOptions,
	FillerBid,
	SelectBidResult,
	TokenInfo,
	Bid,
} from "@/types"
import type { IntentGatewayContext } from "./types"
import { CryptoUtils } from "./CryptoUtils"
import { BidImpl } from "./Bid"
import Decimal from "decimal.js"

/**
 * Manages the solver bid lifecycle for IntentGatewayV2 orders.
 *
 * Responsibilities include:
 * - Constructing signed `PackedUserOperation` objects that solvers submit to the
 *   Hyperbridge coprocessor as bids (`prepareSubmitBid`).
 * - Decoding raw filler bids into first-class {@link Bid} objects (`buildBids`)
 *   that consumers can rank, simulate, and execute themselves.
 * - Sorting bids by output value (`sortBids`) and providing the autopilot
 *   sort-simulate-execute helper (`selectAndExecuteBest`) for consumers that do
 *   not need custom selection logic.
 * - Pricing bid outputs using on-chain DEX quotes so that the highest-value
 *   solver is preferred.
 */
export class BidManager {
	/**
	 * @param ctx - Shared IntentsV2 context providing the destination chain
	 *   client, coprocessor, bundler URL, and session-key storage.
	 * @param crypto - Crypto utilities used for gas packing, UserOp hashing,
	 *   EIP-712 signing, and bundler calls.
	 */
	constructor(
		private readonly ctx: IntentGatewayContext,
		private readonly crypto: CryptoUtils,
	) {}

	/**
	 * Constructs a signed `PackedUserOperation` that a solver can submit to the
	 * Hyperbridge coprocessor as a bid to fill an order.
	 *
	 * The solver signs the operation as EntryPoint v0.8 EIP-712 typed data,
	 * whose digest is the plain userOpHash. The binding to the order lives in
	 * the operation itself: the 4337 nonce key must be the lower 192 bits of
	 * the order commitment (`SolverAccount` enforces this during validation),
	 * and the callData carries the order. This keeps the signed payload fully
	 * transparent to signing infrastructure instead of an opaque digest.
	 *
	 * @param options - Parameters describing the solver account, gas limits, fee
	 *   market values, and pre-built `callData` for the fill operation.
	 * @returns A `PackedUserOperation` with the solver's signature prepended
	 *   with the order commitment.
	 */
	async prepareSubmitBid(options: SubmitBidOptions): Promise<PackedUserOperation> {
		const {
			order,
			solverAccount,
			solverSigner,
			nonce,
			entryPointAddress,
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
			callData,
			paymasterAndData = "0x" as HexString,
		} = options

		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)

		const accountGasLimits = CryptoUtils.packGasLimits(verificationGasLimit, callGasLimit)
		const gasFees = CryptoUtils.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

		const userOp: PackedUserOperation = {
			sender: solverAccount,
			nonce,
			initCode: "0x" as HexString,
			callData,
			accountGasLimits,
			preVerificationGas,
			gasFees,
			paymasterAndData,
			signature: "0x" as HexString,
		}

		// SolverAccount validates this signature against the plain userOpHash and
		// requires the nonce key to bind the order commitment and session key.
		const nonceKey = BigInt(nonce) >> 64n
		const expectedKey = CryptoUtils.bidNonceKey(order.id as HexString, order.session as HexString)
		if (nonceKey !== expectedKey) {
			console.warn(
				`[BidManager] bid nonce key does not bind the order commitment and session key; on-chain validation will fail (order=${order.id})`,
			)
		}
		const solverSignature = await solverSigner.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, entryPointAddress, chainId),
			Number(chainId),
		)

		const signature = concat([order.id as HexString, solverSignature as HexString]) as HexString

		return { ...userOp, signature }
	}

	/**
	 * Decodes raw filler bids into first-class {@link Bid} objects.
	 *
	 * Each bid's `fillOrder` fill-options are decoded from its ERC-7821 calldata;
	 * bids whose calldata cannot be decoded into a valid `fillOrder` call are
	 * silently dropped with a warning. The returned `Bid` instances are ready to
	 * be ranked, simulated, and executed by the consumer.
	 *
	 * @param order - The placed order the bids are competing to fill.
	 * @param bids - Raw filler bids fetched from the coprocessor.
	 * @param sessionPrivateKey - Optional session-key override; looked up from
	 *   storage by `order.session` if omitted.
	 * @returns Array of executable `Bid` objects (one per successfully decoded bid).
	 */
	buildBids(order: Order, bids: FillerBid[], sessionPrivateKey?: HexString): Bid[] {
		const chainId = this.ctx.dest.config.stateMachineId
		const priceOutputs = (outputs: TokenInfo[]) => this.computeOutputsUsdValue(outputs, chainId)

		const result: BidImpl[] = []
		for (const fillerBid of bids) {
			const fillOptions = this.decodeBidFillOptions(fillerBid)
			if (!fillOptions) {
				console.warn(`[BidManager] Failed to decode fillOptions from bid by solver=${fillerBid.userOp.sender}`)
				continue
			}
			result.push(
				new BidImpl({
					ctx: this.ctx,
					crypto: this.crypto,
					order,
					fillerBid,
					fillOptions,
					priceOutputs,
					sessionPrivateKey,
				}),
			)
		}
		console.log(`[BidManager] Built ${result.length}/${bids.length} bid(s) successfully`)
		return result
	}

	/**
	 * Decodes raw filler bids, sorts them, simulates each until one passes, signs
	 * the `SelectSolver` message, and submits — all with no per-bid input from the
	 * caller.
	 *
	 * Equivalent to `selectAndExecuteBest(order, buildBids(order, bids, key))`.
	 *
	 * @param order - The placed order to fill.
	 * @param bids - Raw filler bids fetched from the coprocessor.
	 * @param sessionPrivateKey - Optional session-key override; looked up from
	 *   storage by `order.session` if omitted.
	 * @returns A {@link SelectBidResult} for the executed bid.
	 */
	async selectBid(order: Order, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		return this.selectAndExecuteBest(order, this.buildBids(order, bids, sessionPrivateKey))
	}

	/**
	 * Autopilot bid selection: sorts the given bids by output value, simulates
	 * each in order until one passes, then executes that bid. For consumers that
	 * do not need custom selection logic.
	 *
	 * @param order - The placed order to fill.
	 * @param bids - Candidate bids (from {@link buildBids}).
	 * @returns A {@link SelectBidResult} for the executed bid.
	 * @throws If no valid bids exist, all simulations fail, or the bundler rejects
	 *   the UserOperation.
	 */
	async selectAndExecuteBest(order: Order, bids: Bid[]): Promise<SelectBidResult> {
		const commitment = order.id as HexString
		console.log(`[BidManager] selectAndExecuteBest called for commitment=${commitment}, ${bids.length} bid(s)`)

		if (!this.ctx.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}
		if (!this.ctx.intentsCoprocessor) {
			throw new Error("IntentsCoprocessor required")
		}

		const sortedBids = await this.sortBids(order, bids)
		console.log(`[BidManager] ${sortedBids.length}/${bids.length} bid(s) passed validation and sorting`)
		if (sortedBids.length === 0) {
			throw new Error("No valid bids found")
		}

		console.log(`[BidManager] Simulating ${sortedBids.length} sorted bid(s) to find a valid one`)
		for (let idx = 0; idx < sortedBids.length; idx++) {
			const bid = sortedBids[idx]
			console.log(`[BidManager] Simulating bid ${idx + 1}/${sortedBids.length} from solver=${bid.solverAddress}`)

			try {
				await bid.simulate()
			} catch (err) {
				console.warn(
					`[BidManager] Bid ${idx + 1} from solver=${bid.solverAddress}: simulation FAILED: ` +
						`${err instanceof Error ? err.message : String(err)}`,
				)
				continue
			}

			console.log(`[BidManager] Bid ${idx + 1} from solver=${bid.solverAddress}: simulation PASSED`)
			return bid.execute()
		}

		console.error(`[BidManager] All ${sortedBids.length} bid(s) failed simulation for commitment=${commitment}`)
		throw new Error("No bids passed simulation")
	}

	/**
	 * Sorts a list of bids for the given order by output value.
	 *
	 * Delegates to one of three strategies based on the order's output token
	 * composition:
	 * - Single output token: sort by offered amount descending.
	 * - All stable outputs (USDC/USDT): sort by normalised USD value descending.
	 * - Mixed outputs: sort by DEX-quoted USD value descending, with a raw-amount
	 *   fallback if pricing fails.
	 *
	 * Bids that cannot satisfy the order's token set are dropped.
	 *
	 * @param order - The placed order whose output spec drives sorting logic.
	 * @param bids - Executable bids to sort (from {@link buildBids}).
	 * @returns Sorted array of `Bid` objects ready for simulation.
	 */
	async sortBids(order: Order, bids: Bid[]): Promise<Bid[]> {
		const outputs = order.output.assets

		if (outputs.length <= 1) {
			console.log(`[BidManager] Using single-output sorting (1 output asset)`)
			return this.sortSingleOutput(bids, outputs[0])
		}

		const chainId = this.ctx.dest.config.stateMachineId
		const allStables = outputs.every((o) => this.isStableToken(bytes32ToBytes20(o.token), chainId))

		if (allStables) {
			console.log(`[BidManager] Using all-stables sorting (${outputs.length} stable output assets)`)
			return this.sortAllStables(bids, outputs, chainId)
		}

		console.log(`[BidManager] Using mixed-output sorting (${outputs.length} output assets, some non-stable)`)
		return this.sortMixedOutputs(bids, outputs, chainId)
	}

	/**
	 * Extracts the `FillOptions` struct from a single bid's ERC-7821
	 * batch calldata by finding and decoding the inner `fillOrder` call.
	 *
	 * @param bid - A single filler bid.
	 * @returns The decoded `FillOptions`, or `null` if extraction fails.
	 */
	private decodeBidFillOptions(bid: FillerBid): FillOptions | null {
		try {
			const innerCalls = this.crypto.decodeERC7821Execute(bid.userOp.callData)
			if (!innerCalls || innerCalls.length === 0) return null

			for (const call of innerCalls) {
				try {
					const decoded = decodeFunctionData({
						abi: IntentGatewayV2ABI,
						data: call.data,
					})
					if (decoded?.functionName === "fillOrder" && decoded.args && decoded.args.length >= 2) {
						const fillOptions = decoded.args[1] as FillOptions
						if (fillOptions?.outputs?.length > 0) {
							return fillOptions
						}
					}
				} catch {
					continue
				}
			}
		} catch {
			// decode failed
		}
		return null
	}

	/**
	 * Case A: single output token.
	 * Filter bids by token match only, sort descending by amount.
	 * Partial fill bids are allowed — the contract determines fill status.
	 */
	private sortSingleOutput(bids: Bid[], requiredAsset: TokenInfo): Bid[] {
		const requiredAmount = new Decimal(requiredAsset.amount.toString())
		console.log(
			`[BidManager] sortSingleOutput: required token=${requiredAsset.token}, amount=${requiredAmount.toString()}`,
		)

		const validBids: { bid: Bid; amount: bigint }[] = []

		for (const bid of bids) {
			const bidOutput = bid.outputs[0]
			const bidAmount = new Decimal(bidOutput.amount.toString())

			if (bidOutput.token.toLowerCase() !== requiredAsset.token.toLowerCase()) {
				console.warn(
					`[BidManager] Bid from solver=${bid.solverAddress} REJECTED: token mismatch ` +
						`(bid=${bidOutput.token}, required=${requiredAsset.token})`,
				)
				continue
			}

			if (bidAmount.lt(requiredAmount)) {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress}: partial fill candidate ` +
						`(bid=${bidAmount.toString()}, required=${requiredAmount.toString()}, ` +
						`covers=${bidAmount.div(requiredAmount).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress} ACCEPTED: amount=${bidAmount.toString()} ` +
						`(surplus=${bidAmount.minus(requiredAmount).toString()})`,
				)
			}

			validBids.push({ bid, amount: bidOutput.amount })
		}

		validBids.sort((a, b) => {
			const aAmt = new Decimal(a.amount.toString())
			const bAmt = new Decimal(b.amount.toString())
			return bAmt.comparedTo(aAmt)
		})

		return validBids.map(({ bid }) => bid)
	}

	/**
	 * Case B: all outputs are USDC/USDT.
	 * Sum normalised USD values (treating each stable as $1) and sort descending.
	 * Partial fill bids are allowed.
	 */
	private sortAllStables(bids: Bid[], orderOutputs: TokenInfo[], chainId: string): Bid[] {
		const requiredUsd = this.computeStablesUsdValue(orderOutputs, chainId)
		console.log(`[BidManager] sortAllStables: required USD value=${requiredUsd.toString()}`)

		const validBids: { bid: Bid; usdValue: Decimal }[] = []

		for (const bid of bids) {
			const bidUsd = this.computeStablesUsdValue(bid.outputs, chainId)

			if (bidUsd === null) {
				console.warn(`[BidManager] Bid from solver=${bid.solverAddress} REJECTED: unable to compute USD value`)
				continue
			}

			if (bidUsd.lt(requiredUsd)) {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress}: partial fill candidate ` +
						`(bid=${bidUsd.toString()}, required=${requiredUsd.toString()}, ` +
						`covers=${bidUsd.div(requiredUsd).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(`[BidManager] Bid from solver=${bid.solverAddress} ACCEPTED: USD value=${bidUsd.toString()}`)
			}

			validBids.push({ bid, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ bid }) => bid)
	}

	/**
	 * Case C: mixed output tokens (at least one non-stable).
	 * Price every token via on-chain DEX quotes, fall back to raw amounts
	 * if pricing is unavailable. Partial fill bids are allowed.
	 */
	private async sortMixedOutputs(bids: Bid[], orderOutputs: TokenInfo[], chainId: string): Promise<Bid[]> {
		const requiredUsd = await this.computeOutputsUsdValue(orderOutputs, chainId)

		if (requiredUsd === null) {
			console.warn("[BidManager] sortMixedOutputs: output tokens unpriceable, falling back to raw-amount sort")
			return this.sortByRawAmountFallback(bids, orderOutputs)
		}

		console.log(`[BidManager] sortMixedOutputs: required USD value=${requiredUsd.toString()}`)
		const validBids: { bid: Bid; usdValue: Decimal }[] = []

		for (const bid of bids) {
			const bidUsd = await this.computeOutputsUsdValue(bid.outputs, chainId)

			if (bidUsd === null) {
				console.warn(`[BidManager] Bid from solver=${bid.solverAddress} REJECTED: unable to price mixed outputs`)
				continue
			}

			if (bidUsd.lt(requiredUsd)) {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress}: partial fill candidate ` +
						`(bid=${bidUsd.toString()}, required=${requiredUsd.toString()}, ` +
						`covers=${bidUsd.div(requiredUsd).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress} ACCEPTED: mixed USD value=${bidUsd.toString()}`,
				)
			}

			validBids.push({ bid, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ bid }) => bid)
	}

	/**
	 * Fallback when DEX pricing is unavailable.
	 * Computes total spread per bid. Bids missing a required token are rejected.
	 * Bids offering less than required for a token are allowed (partial fill).
	 * Sorted by total offered amount descending.
	 */
	private sortByRawAmountFallback(bids: Bid[], orderOutputs: TokenInfo[]): Bid[] {
		console.log(
			`[BidManager] sortByRawAmountFallback: checking ${bids.length} bid(s) against ${orderOutputs.length} required output(s)`,
		)
		const validBids: { bid: Bid; totalOffered: Decimal }[] = []

		for (const bid of bids) {
			let valid = true
			let totalOffered = new Decimal(0)
			let rejectReason = ""

			for (const required of orderOutputs) {
				const matching = bid.outputs.find((o) => o.token.toLowerCase() === required.token.toLowerCase())
				if (!matching) {
					valid = false
					rejectReason = `missing output token=${required.token}`
					break
				}
				totalOffered = totalOffered.plus(new Decimal(matching.amount.toString()))
			}

			if (!valid) {
				console.warn(`[BidManager] Bid from solver=${bid.solverAddress} REJECTED (fallback): ${rejectReason}`)
				continue
			}

			const totalRequired = orderOutputs.reduce(
				(acc, o) => acc.plus(new Decimal(o.amount.toString())),
				new Decimal(0),
			)

			if (totalOffered.lt(totalRequired)) {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress}: partial fill candidate (fallback) ` +
						`(offered=${totalOffered.toString()}, required=${totalRequired.toString()}, ` +
						`covers=${totalOffered.div(totalRequired).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.solverAddress} ACCEPTED (fallback): totalOffered=${totalOffered.toString()}`,
				)
			}

			validBids.push({ bid, totalOffered })
		}

		validBids.sort((a, b) => b.totalOffered.comparedTo(a.totalOffered))
		return validBids.map(({ bid }) => bid)
	}

	// ── Token classification helpers ──────────────────────────────────

	/**
	 * Returns `true` if `tokenAddr` is either USDC or USDT on the given chain.
	 *
	 * @param tokenAddr - 20-byte ERC-20 token address (hex).
	 * @param chainId - State-machine ID of the chain to look up token addresses on.
	 */
	private isStableToken(tokenAddr: HexString, chainId: string): boolean {
		const configService = this.ctx.dest.configService
		const usdc = configService.getUsdcAsset(chainId)
		const usdt = configService.getUsdtAsset(chainId)
		const normalized = tokenAddr.toLowerCase()
		return normalized === usdc.toLowerCase() || normalized === usdt.toLowerCase()
	}

	/**
	 * Returns the ERC-20 decimal count for a known stable token (USDC or USDT)
	 * on the given chain.
	 *
	 * @param tokenAddr - 20-byte token address (hex).
	 * @param chainId - State-machine ID of the chain.
	 * @returns Decimal count (e.g. 6 for USDC on most chains).
	 */
	private getStableDecimals(tokenAddr: HexString, chainId: string): number {
		const configService = this.ctx.dest.configService
		if (tokenAddr.toLowerCase() === configService.getUsdcAsset(chainId).toLowerCase()) {
			return configService.getUsdcDecimals(chainId)
		}
		return configService.getUsdtDecimals(chainId)
	}

	// ── Basket valuation helpers ──────────────────────────────────────

	/**
	 * Sums the USD value of a basket of stable tokens (USDC/USDT only),
	 * normalising each amount by its decimal count and treating each token as $1.
	 *
	 * @param outputs - List of token/amount pairs where every token is a stable.
	 * @param chainId - State-machine ID used to look up decimals.
	 * @returns Total USD value as a `Decimal`.
	 */
	private computeStablesUsdValue(outputs: TokenInfo[], chainId: string): Decimal {
		let total = new Decimal(0)
		for (const output of outputs) {
			const tokenAddr = bytes32ToBytes20(output.token)
			const decimals = this.getStableDecimals(tokenAddr, chainId)
			total = total.plus(new Decimal(output.amount.toString()).div(new Decimal(10).pow(decimals)))
		}
		return total
	}

	/**
	 * Prices every token in the basket via on-chain DEX quotes (token → USDC).
	 * Stables are valued at $1. Non-stables are quoted through Uniswap
	 * (direct to USDC, or via WETH→USDC as fallback).
	 * Returns `null` if any token pricing fails.
	 */
	private async computeOutputsUsdValue(
		outputs: { token: HexString; amount: bigint }[],
		chainId: string,
	): Promise<Decimal | null> {
		const configService = this.ctx.dest.configService
		const client = this.ctx.dest.client
		const usdcAddr = configService.getUsdcAsset(chainId)
		const usdcDecimals = configService.getUsdcDecimals(chainId)
		const { asset: wethAddr } = configService.getWrappedNativeAssetWithDecimals(chainId)

		let totalUsd = new Decimal(0)

		for (const output of outputs) {
			const tokenAddr = bytes32ToBytes20(output.token)

			if (this.isStableToken(tokenAddr, chainId)) {
				const decimals = this.getStableDecimals(tokenAddr, chainId)
				totalUsd = totalUsd.plus(new Decimal(output.amount.toString()).div(new Decimal(10).pow(decimals)))
				continue
			}

			try {
				const usdcAmount = await this.quoteTokenToUsdc(
					tokenAddr,
					output.amount,
					wethAddr,
					usdcAddr,
					chainId,
					client,
				)
				totalUsd = totalUsd.plus(new Decimal(usdcAmount.toString()).div(new Decimal(10).pow(usdcDecimals)))
			} catch {
				return null
			}
		}

		return totalUsd
	}

	/**
	 * Gets the USDC-equivalent amount for a non-stable token using on-chain DEX quotes.
	 * Tries direct token→USDC first, then falls back to token→WETH→USDC.
	 */
	private async quoteTokenToUsdc(
		tokenAddr: HexString,
		amount: bigint,
		wethAddr: HexString,
		usdcAddr: HexString,
		chainId: string,
		client: IntentGatewayContext["dest"]["client"],
	): Promise<bigint> {
		const isWethOrNative = tokenAddr.toLowerCase() === wethAddr.toLowerCase() || tokenAddr === ADDRESS_ZERO

		if (isWethOrNative) {
			const { amountOut, protocol } = await this.ctx.swap.findBestProtocolWithAmountIn(
				client,
				wethAddr,
				usdcAddr,
				amount,
				chainId,
			)
			if (protocol === null || amountOut === 0n) throw new Error("No WETH→USDC liquidity")
			return amountOut
		}

		// Try direct: token → USDC
		try {
			const { amountOut, protocol } = await this.ctx.swap.findBestProtocolWithAmountIn(
				client,
				tokenAddr,
				usdcAddr,
				amount,
				chainId,
			)
			if (protocol === null || amountOut === 0n) throw new Error("No direct liquidity")
			return amountOut
		} catch {
			// Fallback: token → WETH → USDC
			const { amountOut: wethOut, protocol: p1 } = await this.ctx.swap.findBestProtocolWithAmountIn(
				client,
				tokenAddr,
				wethAddr,
				amount,
				chainId,
			)
			if (p1 === null || wethOut === 0n) throw new Error("No token→WETH liquidity")

			const { amountOut: usdcOut, protocol: p2 } = await this.ctx.swap.findBestProtocolWithAmountIn(
				client,
				wethAddr,
				usdcAddr,
				wethOut,
				chainId,
			)
			if (p2 === null || usdcOut === 0n) throw new Error("No WETH→USDC liquidity")
			return usdcOut
		}
	}
}
