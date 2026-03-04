import { encodeFunctionData, decodeFunctionData, concat, keccak256, parseEventLogs, erc20Abi } from "viem"
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
} from "@/types"
import type { IntentsV2Context } from "./types"
import { BundlerMethod } from "./types"
import { transformOrderForContract } from "./utils"
import { CryptoUtils } from "./CryptoUtils"
import Decimal from "decimal.js"

export class BidManager {
	constructor(
		private readonly ctx: IntentsV2Context,
		private readonly crypto: CryptoUtils,
	) {}

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

	async selectBid(order: OrderV2, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		const commitment = order.id as HexString
		const sessionKeyAddress = order.session as HexString
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

		for (const bidWithOptions of sortedBids) {
			const solverAddress = bidWithOptions.bid.userOp.sender

			const signature = await this.crypto.signSolverSelection(
				commitment,
				solverAddress,
				domainSeparator,
				sessionKeyData.privateKey,
			)
			if (!signature) {
				continue
			}

			const selectOptions: SelectOptions = {
				commitment,
				solver: solverAddress,
				signature,
			}

			try {
				await this.simulate(order, selectOptions, bidWithOptions.options, solverAddress, intentGatewayV2Address)
				selectedBid = bidWithOptions
				sessionSignature = signature
				break
			} catch (err) {
				console.debug(
					"Bid simulation failed",
					JSON.stringify({
						commitment,
						solver: solverAddress,
						error: err instanceof Error ? err.message : String(err),
					}),
				)
				continue
			}
		}

		if (!selectedBid || !sessionSignature) {
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
					const chainReceipt = await this.ctx.dest.client.getTransactionReceipt({
						hash: txnHash,
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
					}
				} catch {
					throw new Error("Failed to determine fill status from logs")
				}
			}
		} catch {
			// Receipt may not be available
		}

		return {
			userOp: signedUserOp,
			userOpHash,
			solverAddress,
			commitment,
			txnHash,
			fillStatus,
		}
	}

	private async validateAndSortBids(
		bids: FillerBid[],
		order: OrderV2,
	): Promise<{ bid: FillerBid; options: FillOptionsV2 }[]> {
		const outputs = order.output.assets
		const decodedBids = this.decodeBids(bids)

		if (outputs.length <= 1) {
			return this.sortSingleOutput(decodedBids, outputs[0])
		}

		const chainId = this.ctx.dest.config.stateMachineId
		const allStables = outputs.every((o) => this.isStableToken(bytes32ToBytes20(o.token), chainId))

		if (allStables) {
			return this.sortAllStables(decodedBids, outputs, chainId)
		}

		return this.sortMixedOutputs(decodedBids, outputs, chainId)
	}

	private decodeBids(bids: FillerBid[]): { bid: FillerBid; options: FillOptionsV2 }[] {
		const result: { bid: FillerBid; options: FillOptionsV2 }[] = []
		for (const bid of bids) {
			const fillOptions = this.decodeBidFillOptions(bid)
			if (fillOptions) {
				result.push({ bid, options: fillOptions })
			}
		}
		return result
	}

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

	private async simulate(
		order: OrderV2,
		selectOptions: SelectOptions,
		fillOptions: FillOptionsV2,
		solverAddress: HexString,
		intentGatewayV2Address: HexString,
	): Promise<void> {
		const nativeOutputValue = order.output.assets
			.filter((asset) => bytes32ToBytes20(asset.token) === ADDRESS_ZERO)
			.reduce((sum, asset) => sum + asset.amount, 0n)
		const totalNativeValue = nativeOutputValue + fillOptions.nativeDispatchFee

		const selectCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "select",
			args: [selectOptions],
		}) as HexString

		const fillOrderCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [transformOrderForContract(order), fillOptions],
		}) as HexString

		const batchedCalldata = this.crypto.encodeERC7821Execute([
			{
				target: intentGatewayV2Address,
				value: 0n,
				data: selectCalldata,
			},
			{
				target: intentGatewayV2Address,
				value: totalNativeValue,
				data: fillOrderCalldata,
			},
		])

		try {
			await this.ctx.dest.client.call({
				account: solverAddress,
				to: solverAddress,
				data: batchedCalldata,
				value: totalNativeValue,
			})
		} catch (e: unknown) {
			throw new Error(`Simulation failed: ${e instanceof Error ? e.message : String(e)}`)
		}
	}

	/**
	 * Case A: single output token – keep existing behavior.
	 * Filter bids whose first output amount < required, sort descending.
	 */
	private sortSingleOutput(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		requiredAsset: TokenInfoV2,
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		const validBids: { bid: FillerBid; options: FillOptionsV2; amount: bigint }[] = []

		for (const { bid, options } of decodedBids) {
			const bidOutput = options.outputs[0]
			const bidAmount = new Decimal(bidOutput.amount.toString())
			const requiredAmount = new Decimal(requiredAsset.amount.toString())
			if (bidAmount.lt(requiredAmount)) continue
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
	 * Sum normalised USD values (treating each stable as $1) and compare.
	 */
	private sortAllStables(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
		chainId: string,
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		const requiredUsd = this.computeStablesUsdValue(orderOutputs, chainId)
		const validBids: { bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			const bidUsd = this.computeStablesUsdValue(options.outputs, chainId)
			if (bidUsd === null || bidUsd.lt(requiredUsd)) continue
			validBids.push({ bid, options, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ usdValue: _, ...rest }) => rest)
	}

	/**
	 * Case C: mixed output tokens (at least one non-stable).
	 * Price every token via on-chain DEX quotes, fall back to raw amounts
	 * if pricing is unavailable.
	 */
	private async sortMixedOutputs(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
		chainId: string,
	): Promise<{ bid: FillerBid; options: FillOptionsV2 }[]> {
		const requiredUsd = await this.computeOutputsUsdValue(orderOutputs, chainId)

		if (requiredUsd === null) {
			console.warn("BidManager: output tokens unpriceable, falling back to raw-amount sort")
			return this.sortByRawAmountFallback(decodedBids, orderOutputs)
		}

		const validBids: { bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			const bidUsd = await this.computeOutputsUsdValue(options.outputs, chainId)
			if (bidUsd === null || bidUsd.lt(requiredUsd)) continue
			validBids.push({ bid, options, usdValue: bidUsd })
		}

		validBids.sort((a, b) => b.usdValue.comparedTo(a.usdValue))
		return validBids.map(({ usdValue: _, ...rest }) => rest)
	}

	/**
	 * Fallback when DEX pricing is unavailable.
	 * Computes the total spread (sum of extra amount above required per token)
	 * for each bid. Bids that don't meet every token's minimum are discarded.
	 * Remaining bids are sorted by total spread descending.
	 */
	private sortByRawAmountFallback(
		decodedBids: { bid: FillerBid; options: FillOptionsV2 }[],
		orderOutputs: TokenInfoV2[],
	): { bid: FillerBid; options: FillOptionsV2 }[] {
		const validBids: { bid: FillerBid; options: FillOptionsV2; totalSpread: Decimal }[] = []

		for (const { bid, options } of decodedBids) {
			let valid = true
			let totalSpread = new Decimal(0)

			for (const required of orderOutputs) {
				const matching = options.outputs.find((o) => o.token.toLowerCase() === required.token.toLowerCase())
				if (!matching || matching.amount < required.amount) {
					valid = false
					break
				}
				totalSpread = totalSpread.plus(
					new Decimal(matching.amount.toString()).minus(required.amount.toString()),
				)
			}
			if (!valid) continue

			validBids.push({ bid, options, totalSpread })
		}

		validBids.sort((a, b) => b.totalSpread.comparedTo(a.totalSpread))
		return validBids.map(({ totalSpread: _, ...rest }) => rest)
	}

	// ── Token classification helpers ──────────────────────────────────

	private isStableToken(tokenAddr: HexString, chainId: string): boolean {
		const configService = this.ctx.dest.configService
		const usdc = configService.getUsdcAsset(chainId)
		const usdt = configService.getUsdtAsset(chainId)
		const normalized = tokenAddr.toLowerCase()
		return normalized === usdc.toLowerCase() || normalized === usdt.toLowerCase()
	}

	private getStableDecimals(tokenAddr: HexString, chainId: string): number {
		const configService = this.ctx.dest.configService
		if (tokenAddr.toLowerCase() === configService.getUsdcAsset(chainId).toLowerCase()) {
			return configService.getUsdcDecimals(chainId)
		}
		return configService.getUsdtDecimals(chainId)
	}

	// ── Basket valuation helpers ──────────────────────────────────────

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
