import { encodeFunctionData, decodeFunctionData, concat, keccak256, parseEventLogs } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { ADDRESS_ZERO, bytes32ToBytes20, hexToString, retryPromise } from "@/utils"
import type {
	OrderV2,
	HexString,
	PackedUserOperation,
	SubmitBidOptions,
	FillOptionsV2,
	SelectOptions,
	FillerBid,
	SelectBidResult,
	TokenInfoV2,
	ERC7821Call,
} from "@/types"
import type { IntentsV2Context } from "./types"
import { BundlerMethod } from "./types"
import { CryptoUtils } from "./CryptoUtils"
import Decimal from "decimal.js"

/**
 * Manages the solver bid lifecycle for IntentGatewayV2 orders.
 *
 * Responsibilities include:
 * - Constructing signed `PackedUserOperation` objects that solvers submit to the
 *   Hyperbridge coprocessor as bids (`prepareSubmitBid`).
 * - Validating, sorting, simulating, and submitting the best available bid for
 *   a given order (`selectBid`).
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
		private readonly ctx: IntentsV2Context,
		private readonly crypto: CryptoUtils,
	) {}

	/**
	 * Constructs a signed `PackedUserOperation` that a solver can submit to the
	 * Hyperbridge coprocessor as a bid to fill an order.
	 *
	 * The solver's signature covers a hash that binds the UserOperation to the
	 * order commitment and the session key address, so the IntentGatewayV2
	 * contract can verify the solver's intent on-chain.
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
			solverPrivateKey,
			nonce,
			entryPointAddress,
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
			callData,
		} = options

		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)

		const accountGasLimits = this.crypto.packGasLimits(verificationGasLimit, callGasLimit)
		const gasFees = this.crypto.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

		const userOp: PackedUserOperation = {
			sender: solverAccount,
			nonce,
			initCode: "0x" as HexString,
			callData,
			accountGasLimits,
			preVerificationGas,
			gasFees,
			paymasterAndData: "0x" as HexString,
			signature: "0x" as HexString,
		}

		const userOpHash = this.crypto.computeUserOpHash(userOp, entryPointAddress, chainId)
		const sessionKey = order.session

		const messageHash = keccak256(concat([userOpHash, order.id as HexString, sessionKey as import("viem").Hex]))

		const solverAccount_ = privateKeyToAccount(solverPrivateKey as import("viem").Hex)
		const solverSignature = await solverAccount_.signMessage({ message: { raw: messageHash } })

		const signature = concat([order.id as HexString, solverSignature as import("viem").Hex]) as HexString

		return { ...userOp, signature }
	}

	/**
	 * Selects the best available bid, simulates it on-chain, signs the
	 * solver-selection EIP-712 message with the session key, and submits the
	 * UserOperation to the bundler.
	 *
	 * **Selection algorithm:**
	 * 1. Decodes `fillOrder` calldata from each bid's `callData`.
	 * 2. Sorts bids by output value (single-output: amount; all-stables: normalised
	 *    USD; mixed: DEX-quoted USD; fallback: raw amount).
	 * 3. Iterates sorted bids, simulating each with `eth_call` until one passes.
	 * 4. Appends the session-key's `SelectSolver` signature to the solver's
	 *    existing signature and submits via `eth_sendUserOperation`.
	 * 5. For same-chain orders, waits for the transaction receipt and reads
	 *    `OrderFilled` / `PartialFill` events to determine fill status.
	 *
	 * @param order - The placed order for which to select a bid.
	 * @param bids - Raw bids fetched from the Hyperbridge coprocessor.
	 * @param sessionPrivateKey - Optional override; if omitted, the key is
	 *   looked up from `sessionKeyStorage` using `order.session`.
	 * @returns A {@link SelectBidResult} containing the submitted UserOperation,
	 *   its hash, the winning solver address, transaction hash, and fill status.
	 * @throws If the session key is not found, no valid bids exist, all
	 *   simulations fail, or the bundler rejects the UserOperation.
	 */
	async selectBid(order: OrderV2, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		const commitment = order.id as HexString
		const sessionKeyAddress = order.session as HexString
		console.log(`[BidManager] selectBid called for commitment=${commitment}, received ${bids.length} bid(s)`)

		const sessionKeyData = sessionPrivateKey
			? { privateKey: sessionPrivateKey as HexString }
			: await this.ctx.sessionKeyStorage.getSessionKeyByAddress(sessionKeyAddress)
		if (!sessionKeyData) {
			throw new Error("SessionKey not found for commitment: " + commitment)
		}

		if (!this.ctx.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		if (!this.ctx.intentsCoprocessor) {
			throw new Error("IntentsCoprocessor required")
		}

		const sortedBids = await this.validateAndSortBids(bids, order)
		console.log(`[BidManager] ${sortedBids.length}/${bids.length} bid(s) passed validation and sorting`)
		if (sortedBids.length === 0) {
			throw new Error("No valid bids found")
		}

		const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayV2Address(
			hexToString(order.destination as HexString),
		)

		const domainSeparator = this.crypto.getDomainSeparator(
			"IntentGateway",
			"2",
			BigInt(
				this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
			),
			intentGatewayV2Address,
		)

		let selectedBid: { bid: FillerBid; options: FillOptionsV2 } | null = null
		let sessionSignature: HexString | null = null

		console.log(`[BidManager] Simulating ${sortedBids.length} sorted bid(s) to find a valid one`)
		for (let idx = 0; idx < sortedBids.length; idx++) {
			const bidWithOptions = sortedBids[idx]
			const solverAddress = bidWithOptions.bid.userOp.sender
			console.log(`[BidManager] Simulating bid ${idx + 1}/${sortedBids.length} from solver=${solverAddress}`)

			const signature = await this.crypto.signSolverSelection(
				commitment,
				solverAddress,
				domainSeparator,
				sessionKeyData.privateKey,
			)
			if (!signature) {
				console.warn(`[BidManager] Bid ${idx + 1}: failed to sign solver selection, skipping`)
				continue
			}

			const selectOptions: SelectOptions = {
				commitment,
				solver: solverAddress,
				signature,
			}

			try {
				await this.simulate(bidWithOptions.bid, selectOptions, intentGatewayV2Address)
				console.log(`[BidManager] Bid ${idx + 1} from solver=${solverAddress}: simulation PASSED`)
				selectedBid = bidWithOptions
				sessionSignature = signature
				break
			} catch (err) {
				console.warn(
					`[BidManager] Bid ${idx + 1} from solver=${solverAddress}: simulation FAILED: ` +
						`${err instanceof Error ? err.message : String(err)}`,
				)
				continue
			}
		}

		if (!selectedBid || !sessionSignature) {
			console.error(`[BidManager] All ${sortedBids.length} bid(s) failed simulation for commitment=${commitment}`)
			throw new Error("No bids passed simulation")
		}

		const solverAddress = selectedBid.bid.userOp.sender

		const finalSignature = concat([
			selectedBid.bid.userOp.signature as import("viem").Hex,
			sessionSignature as import("viem").Hex,
		]) as HexString

		const signedUserOp: PackedUserOperation = {
			...selectedBid.bid.userOp,
			signature: finalSignature,
		}

		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(
			hexToString(order.destination as HexString),
		)
		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)

		const bundlerResult = await this.crypto.sendBundler<HexString>(BundlerMethod.ETH_SEND_USER_OPERATION, [
			this.crypto.prepareBundlerCall(signedUserOp),
			entryPointAddress,
		])

		const userOpHash = bundlerResult

		let txnHash: HexString | undefined
		let fillStatus: "full" | "partial" | undefined
		let filledAmount: bigint | undefined
		try {
			const receipt = await retryPromise(
				async () => {
					const result = await this.crypto.sendBundler<{
						receipt: { transactionHash: HexString }
					} | null>(BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT, [userOpHash])
					if (!result?.receipt?.transactionHash) {
						throw new Error("Receipt not available yet")
					}
					return result
				},
				{ maxRetries: 5, backoffMs: 2000, logMessage: "Fetching user operation receipt" },
			)
			txnHash = receipt.receipt.transactionHash

			if (order.source === order.destination) {
				try {
					const chainReceipt = await this.ctx.dest.client.waitForTransactionReceipt({
						hash: txnHash,
						confirmations: 1,
					})
					const events = parseEventLogs({
						abi: IntentGatewayV2ABI,
						logs: chainReceipt.logs,
						eventName: ["OrderFilled", "PartialFill"],
					})

					const matched = events.find((e) => {
						if (e.eventName === "OrderFilled")
							return e.args.commitment.toLowerCase() === commitment.toLowerCase()
						if (e.eventName === "PartialFill")
							return e.args.commitment.toLowerCase() === commitment.toLowerCase()
						return false
					})

					if (matched?.eventName === "OrderFilled") {
						fillStatus = "full"
					} else if (matched?.eventName === "PartialFill") {
						fillStatus = "partial"

						// Sum all output amounts from the PartialFill event as the filled amount for this attempt
						const outputs = (matched.args.outputs ?? []) as readonly { amount: bigint }[]
						filledAmount = outputs.reduce((acc, o) => acc + o.amount, 0n)
					}
				} catch {
					throw new Error("Failed to determine fill status from logs")
				}
			}
		} catch (err) {
			// Receipt may not be available
			throw new Error(`Failed to select bid: ${err instanceof Error ? err.message : String(err)}`)
		}

		return {
			userOp: signedUserOp,
			userOpHash,
			solverAddress,
			commitment,
			txnHash,
			fillStatus,
			filledAmount,
		}
	}

	/**
	 * Validates and sorts a list of raw bids for the given order.
	 *
	 * Delegates to one of three strategies based on the order's output token
	 * composition:
	 * - Single output token: sort by offered amount descending.
	 * - All stable outputs (USDC/USDT): sort by normalised USD value descending.
	 * - Mixed outputs: sort by DEX-quoted USD value descending, with a raw-amount
	 *   fallback if pricing fails.
	 *
	 * @param bids - Raw filler bids from the coprocessor.
	 * @param order - The placed order whose output spec drives sorting logic.
	 * @returns Sorted array of `{ bid, options }` pairs ready for simulation.
	 */
	private async validateAndSortBids(
		bids: FillerBid[],
		order: OrderV2,
	): Promise<{ bid: FillerBid; options: FillOptionsV2 }[]> {
		const outputs = order.output.assets
		const decodedBids = this.decodeBids(bids)

		if (outputs.length <= 1) {
			console.log(`[BidManager] Using single-output sorting (1 output asset)`)
			return this.sortSingleOutput(decodedBids, outputs[0])
		}

		const chainId = this.ctx.dest.config.stateMachineId
		const allStables = outputs.every((o) => this.isStableToken(bytes32ToBytes20(o.token), chainId))

		if (allStables) {
			console.log(`[BidManager] Using all-stables sorting (${outputs.length} stable output assets)`)
			return this.sortAllStables(decodedBids, outputs, chainId)
		}

		console.log(`[BidManager] Using mixed-output sorting (${outputs.length} output assets, some non-stable)`)
		return this.sortMixedOutputs(decodedBids, outputs, chainId)
	}

	/**
	 * Decodes the `fillOrder` fill-options from each bid's ERC-7821 calldata.
	 *
	 * Bids whose calldata cannot be decoded or do not contain a valid
	 * `fillOrder` call are silently dropped with a warning.
	 *
	 * @param bids - Raw bids to decode.
	 * @returns Array of successfully decoded `{ bid, options }` pairs.
	 */
	private decodeBids(bids: FillerBid[]): { bid: FillerBid; options: FillOptionsV2 }[] {
		const result: { bid: FillerBid; options: FillOptionsV2 }[] = []
		for (const bid of bids) {
			const fillOptions = this.decodeBidFillOptions(bid)
			if (fillOptions) {
				result.push({ bid, options: fillOptions })
			} else {
				console.warn(`[BidManager] Failed to decode fillOptions from bid by solver=${bid.userOp.sender}`)
			}
		}
		console.log(`[BidManager] Decoded ${result.length}/${bids.length} bid(s) successfully`)
		return result
	}

	/**
	 * Extracts the `FillOptionsV2` struct from a single bid's ERC-7821
	 * batch calldata by finding and decoding the inner `fillOrder` call.
	 *
	 * @param bid - A single filler bid.
	 * @returns The decoded `FillOptionsV2`, or `null` if extraction fails.
	 */
	private decodeBidFillOptions(bid: FillerBid): FillOptionsV2 | null {
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
						const fillOptions = decoded.args[1] as FillOptionsV2
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
	 * Simulates a bid on-chain by batching the `select` and `fillOrder` calls
	 * via `eth_call` from the solver's account, using the IntentGatewayV2
	 * ERC-7821 batch-execute pattern.
	 *
	 * @param bid - The filler bid to simulate.
	 * @param selectOptions - The signed solver-selection parameters.
	 * @param intentGatewayV2Address - Address of the IntentGatewayV2 contract on the destination chain.
	 * @throws If the `eth_call` simulation reverts or errors.
	 */
	private async simulate(
		bid: FillerBid,
		selectOptions: SelectOptions,
		intentGatewayV2Address: HexString,
	): Promise<void> {
		const solverAddress = bid.userOp.sender

		const selectCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "select",
			args: [selectOptions],
		}) as HexString

		const calls: ERC7821Call[] = [
			{ target: intentGatewayV2Address, value: 0n, data: selectCalldata },
			{ target: solverAddress, value: 0n, data: bid.userOp.callData },
		]
		const batchedCalldata = this.crypto.encodeERC7821Execute(calls)

		try {
			await this.ctx.dest.client.call({
				account: solverAddress,
				to: solverAddress,
				data: batchedCalldata,
				value: 0n,
			})
		} catch (e: unknown) {
			throw new Error(`Simulation failed: ${e instanceof Error ? e.message : String(e)}`)
		}
	}

	/**
	 * Case A: single output token.
	 * Filter bids by token match only, sort descending by amount.
	 * Partial fill bids are allowed — the contract determines fill status.
	 */
	private sortSingleOutput(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		requiredAsset: TokenInfoV2,
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		const requiredAmount = new Decimal(requiredAsset.amount.toString())
		console.log(
			`[BidManager] sortSingleOutput: required token=${requiredAsset.token}, amount=${requiredAmount.toString()}`,
		)

		const validBids: { bid: FillerBid; options: FillOptionsV2; amount: bigint }[] = []

		for (const { bid, options } of decodedBids) {
			const bidOutput = options.outputs[0]
			const bidAmount = new Decimal(bidOutput.amount.toString())

			if (bidOutput.token.toLowerCase() !== requiredAsset.token.toLowerCase()) {
				console.warn(
					`[BidManager] Bid from solver=${bid.userOp.sender} REJECTED: token mismatch ` +
						`(bid=${bidOutput.token}, required=${requiredAsset.token})`,
				)
				continue
			}

			if (bidAmount.lt(requiredAmount)) {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender}: partial fill candidate ` +
						`(bid=${bidAmount.toString()}, required=${requiredAmount.toString()}, ` +
						`covers=${bidAmount.div(requiredAmount).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender} ACCEPTED: amount=${bidAmount.toString()} ` +
						`(surplus=${bidAmount.minus(requiredAmount).toString()})`,
				)
			}

			validBids.push({ bid, options, amount: bidOutput.amount })
		}

		validBids.sort((a, b) => {
			const aAmt = new Decimal(a.amount.toString())
			const bAmt = new Decimal(b.amount.toString())
			return bAmt.comparedTo(aAmt)
		})

		return validBids.map(({ amount: _, ...rest }) => rest)
	}

	/**
	 * Case B: all outputs are USDC/USDT.
	 * Sum normalised USD values (treating each stable as $1) and sort descending.
	 * Partial fill bids are allowed.
	 */
	private sortAllStables(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
		chainId: string,
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		const requiredUsd = this.computeStablesUsdValue(orderOutputs, chainId)
		console.log(`[BidManager] sortAllStables: required USD value=${requiredUsd.toString()}`)

		const validBids: { bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			const bidUsd = this.computeStablesUsdValue(options.outputs, chainId)

			if (bidUsd === null) {
				console.warn(`[BidManager] Bid from solver=${bid.userOp.sender} REJECTED: unable to compute USD value`)
				continue
			}

			if (bidUsd.lt(requiredUsd)) {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender}: partial fill candidate ` +
						`(bid=${bidUsd.toString()}, required=${requiredUsd.toString()}, ` +
						`covers=${bidUsd.div(requiredUsd).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender} ACCEPTED: USD value=${bidUsd.toString()}`,
				)
			}

			validBids.push({ bid, options, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ usdValue: _, ...rest }) => rest)
	}

	/**
	 * Case C: mixed output tokens (at least one non-stable).
	 * Price every token via on-chain DEX quotes, fall back to raw amounts
	 * if pricing is unavailable. Partial fill bids are allowed.
	 */
	private async sortMixedOutputs(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
		chainId: string,
	): Promise<{ bid: FillerBid; options: FillOptionsV2 }[]> {
		const requiredUsd = await this.computeOutputsUsdValue(orderOutputs, chainId)

		if (requiredUsd === null) {
			console.warn("[BidManager] sortMixedOutputs: output tokens unpriceable, falling back to raw-amount sort")
			return this.sortByRawAmountFallback(decodedBids, orderOutputs)
		}

		console.log(`[BidManager] sortMixedOutputs: required USD value=${requiredUsd.toString()}`)
		const validBids: { bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			const bidUsd = await this.computeOutputsUsdValue(options.outputs, chainId)

			if (bidUsd === null) {
				console.warn(
					`[BidManager] Bid from solver=${bid.userOp.sender} REJECTED: unable to price mixed outputs`,
				)
				continue
			}

			if (bidUsd.lt(requiredUsd)) {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender}: partial fill candidate ` +
						`(bid=${bidUsd.toString()}, required=${requiredUsd.toString()}, ` +
						`covers=${bidUsd.div(requiredUsd).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender} ACCEPTED: mixed USD value=${bidUsd.toString()}`,
				)
			}

			validBids.push({ bid, options, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ usdValue: _, ...rest }) => rest)
	}

	/**
	 * Fallback when DEX pricing is unavailable.
	 * Computes total spread per bid. Bids missing a required token are rejected.
	 * Bids offering less than required for a token are allowed (partial fill).
	 * Sorted by total offered amount descending.
	 */
	private sortByRawAmountFallback(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		console.log(
			`[BidManager] sortByRawAmountFallback: checking ${decodedBids.length} bid(s) against ${orderOutputs.length} required output(s)`,
		)
		const validBids: { bid: FillerBid; options: FillOptionsV2; totalOffered: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			let valid = true
			let totalOffered = new Decimal(0)
			let rejectReason = ""

			for (const required of orderOutputs) {
				const matching = options.outputs.find((o) => o.token.toLowerCase() === required.token.toLowerCase())
				if (!matching) {
					valid = false
					rejectReason = `missing output token=${required.token}`
					break
				}
				totalOffered = totalOffered.plus(new Decimal(matching.amount.toString()))
			}

			if (!valid) {
				console.warn(`[BidManager] Bid from solver=${bid.userOp.sender} REJECTED (fallback): ${rejectReason}`)
				continue
			}

			const totalRequired = orderOutputs.reduce(
				(acc, o) => acc.plus(new Decimal(o.amount.toString())),
				new Decimal(0),
			)

			if (totalOffered.lt(totalRequired)) {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender}: partial fill candidate (fallback) ` +
						`(offered=${totalOffered.toString()}, required=${totalRequired.toString()}, ` +
						`covers=${totalOffered.div(totalRequired).mul(100).toFixed(2)}%)`,
				)
			} else {
				console.log(
					`[BidManager] Bid from solver=${bid.userOp.sender} ACCEPTED (fallback): totalOffered=${totalOffered.toString()}`,
				)
			}

			validBids.push({ bid, options, totalOffered })
		}

		validBids.sort((a, b) => b.totalOffered.comparedTo(a.totalOffered))
		return validBids.map(({ totalOffered: _, ...rest }) => rest)
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
	private computeStablesUsdValue(outputs: TokenInfoV2[], chainId: string): Decimal {
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
		client: IntentsV2Context["dest"]["client"],
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
