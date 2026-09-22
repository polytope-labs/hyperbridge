import { decodeFunctionData, concat } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { ADDRESS_ZERO, bytes32ToBytes20, normalizeStateMachineId } from "@/utils"
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
import { decodeFillOrder, supportsRateFills, FILL_ORDER_SELECTOR } from "./fillOrderCodec"
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
 * - Sorting bids by rate (`sortBids`) and providing the autopilot
 *   sort-simulate-execute helper (`selectAndExecuteBest`) for consumers that do
 *   not need custom selection logic.
 * - Pricing bid outputs in USD using on-chain DEX quotes, for
 *   {@link Bid.outputUsdValue}.
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
		// Every fillOrder call in the bid must carry a quote for each leg, and the destination gateway
		// must speak the release that settles it.
		const fills = (this.crypto.decodeERC7821Execute(callData) ?? []).filter(
			(call) => call.data.slice(0, 10).toLowerCase() === FILL_ORDER_SELECTOR,
		)
		if (fills.length === 0) throw new Error("Bid calldata carries no fillOrder call")
		for (const call of fills) {
			if (!decodeFillOrder(call.data as HexString)) {
				throw new Error("Malformed fill quote; every leg requires inputs and outputs")
			}
		}
		const chain = normalizeStateMachineId(order.destination)
		const gateway = this.ctx.dest.configService.getIntentGatewayAddress(chain)
		if (!(await supportsRateFills(this.ctx.dest.client as any, gateway))) {
			throw new Error("Fills are not supported by the destination gateway")
		}

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
		// requires the nonce key to bind the order commitment, session key and calldata.
		const nonceKey = BigInt(nonce) >> 64n
		const expectedKey = CryptoUtils.bidNonceKey(order.id as HexString, order.session as HexString, callData)
		if (nonceKey !== expectedKey) {
			console.warn(
				`[BidManager] bid nonce key does not bind the order commitment, session key and calldata; on-chain validation will fail (order=${order.id})`,
			)
		}
		const solverSignature = await solverSigner.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, entryPointAddress, chainId),
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
	 * Autopilot bid selection: sorts the given bids by rate, simulates
	 * each in order and attempts execution. Any bid that fails simulation or
	 * execution is skipped once so the next ranked bid can be attempted in the
	 * same round. For consumers that do not need custom selection logic.
	 *
	 * @param order - The placed order to fill.
	 * @param bids - Candidate bids (from {@link buildBids}).
	 * @returns A {@link SelectBidResult} for the executed bid.
	 * @throws If no valid bids exist or every bid fails simulation/execution.
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
		let simulationFailures = 0
		let executionFailures = 0
		for (let idx = 0; idx < sortedBids.length; idx++) {
			const bid = sortedBids[idx]
			console.log(`[BidManager] Simulating bid ${idx + 1}/${sortedBids.length} from solver=${bid.solverAddress}`)

			try {
				await bid.simulate()
			} catch (err) {
				simulationFailures += 1
				console.warn(
					`[BidManager] Bid ${idx + 1} from solver=${bid.solverAddress}: simulation FAILED: ` +
						`${err instanceof Error ? err.message : String(err)}`,
				)
				continue
			}

			console.log(`[BidManager] Bid ${idx + 1} from solver=${bid.solverAddress}: simulation PASSED`)
			try {
				return await bid.execute()
			} catch (err) {
				executionFailures += 1
				console.warn(
					`[BidManager] Bid ${idx + 1} from solver=${bid.solverAddress}: execution FAILED: ` +
						`${err instanceof Error ? err.message : String(err)}; trying next bid`,
				)
			}
		}

		console.error(
			`[BidManager] No executable bids for commitment=${commitment}: ` +
				`${simulationFailures} simulation failure(s), ${executionFailures} execution failure(s)`,
		)
		throw new Error("No bids passed simulation and execution")
	}

	/**
	 * Sorts bids by rate, best to worst, independently of how much each one fills.
	 *
	 * A leg's rate is what the bid pays out for what it takes: `output / input` for
	 * that leg. Every leg of an order trades the same pair, so a bid's rate is its
	 * total output over its total input across the legs it quotes. Rates are
	 * compared exactly, by cross-multiplication; equal rates keep arrival order.
	 *
	 * A bid is dropped when it does not quote the order's legs one for one, names
	 * another token on a leg, quotes an input without an output (or the reverse),
	 * quotes no leg at all, or quotes a leg below the rate the order requires.
	 *
	 * @param order - The placed order the bids compete to fill.
	 * @param bids - Executable bids to sort (from {@link buildBids}).
	 * @returns The valid bids, best rate first, ready for simulation.
	 */
	async sortBids(order: Order, bids: Bid[]): Promise<Bid[]> {
		const legs = order.inputs.length
		const ranked: { bid: Bid; input: bigint; output: bigint; index: number }[] = []

		for (const [index, bid] of bids.entries()) {
			if (bid.inputs.length !== legs || bid.outputs.length !== legs || order.output.assets.length !== legs) {
				console.warn(`[BidManager] Bid from solver=${bid.solverAddress} REJECTED: does not quote the order's legs`)
				continue
			}
			let input = 0n
			let output = 0n
			let reason = ""
			for (let leg = 0; leg < legs && !reason; leg++) {
				const take = bid.inputs[leg]
				const pay = bid.outputs[leg]
				const escrow = order.inputs[leg]
				const required = order.output.assets[leg]
				if (take.token.toLowerCase() !== escrow.token.toLowerCase()) reason = `input token mismatch on leg ${leg}`
				else if (pay.token.toLowerCase() !== required.token.toLowerCase()) reason = `output token mismatch on leg ${leg}`
				else if (take.amount < 0n || pay.amount < 0n || (take.amount === 0n) !== (pay.amount === 0n))
					reason = `no valid quote on leg ${leg}`
				// output / take >= required / escrow: the leg's rate clears what the user asked for.
				else if (pay.amount * escrow.amount < take.amount * required.amount) reason = `rate below the order on leg ${leg}`
				input += take.amount
				output += pay.amount
			}
			if (!reason && input === 0n) reason = "quotes no leg"
			if (reason) {
				console.warn(`[BidManager] Bid from solver=${bid.solverAddress} REJECTED: ${reason}`)
				continue
			}
			ranked.push({ bid, input, output, index })
		}

		ranked.sort((a, b) => {
			const left = a.output * b.input
			const right = b.output * a.input
			return left === right ? a.index - b.index : left > right ? -1 : 1
		})
		return ranked.map(({ bid }) => bid)
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
				// Bids may be built against either FillOptions shape depending on the gateway
				// the bidder targeted, so decode tries both. A v1 payload reports validUntil
				// as 0n, which is accurate: that fill genuinely carries no bound.
				const decoded = decodeFillOrder(call.data as HexString)
				if (decoded && decoded.options?.outputs?.length > 0) {
					return decoded.options
				}
			}
		} catch {
			// decode failed
		}
		return null
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
			if (output.amount === 0n) continue
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
