import { PublicClient, createPublicClient, http, maxUint256 } from "viem"
import { ChainConfigService } from "@/configs/ChainConfigService"
import type { HexString, Transaction } from "@/types"
import { normalizeEvmChainId } from "@/utils"

export type UniswapProtocol = "v2" | "v3" | "v4"
export type UniswapTradeType = "EXACT_INPUT" | "EXACT_OUTPUT"

/**
 * Token metadata required for Uniswap quoting.
 */
export interface UniswapQuoteToken {
	/** Token contract address. Use ADDRESS_ZERO for the native asset. */
	address: HexString
	/** Token decimals used by the integrating app when formatting quote amounts. */
	decimals: number
	/** Optional display symbol, returned unchanged in quote responses. */
	symbol?: string
	/** EVM chain ID where this token exists. */
	chainId: number
}

/**
 * Parameters for quoting a Uniswap swap through the SDK.
 *
 * Provide `amountIn` when `tradeType` is `EXACT_INPUT`, or `amountOut` when
 * `tradeType` is `EXACT_OUTPUT`. RPC clients are derived from SDK chain config.
 */
export interface QuoteUniswapParams {
	/** Numeric EVM chain ID or state machine ID, for example `8453` or `EVM-8453`. */
	chainId: number | string
	/** Token the user pays with. */
	tokenIn: UniswapQuoteToken
	/** Token the user receives. */
	tokenOut: UniswapQuoteToken
	/** Exact input amount, required for `EXACT_INPUT`. */
	amountIn?: bigint
	/** Exact output amount, required for `EXACT_OUTPUT`. */
	amountOut?: bigint
	/** Whether to quote by fixed input or fixed output. */
	tradeType: UniswapTradeType
	/** Protocols to check. Defaults to all supported protocols: v2, v3, and v4. */
	protocols?: UniswapProtocol[]
	/** Slippage tolerance in basis points, only used when `recipient` is provided. */
	slippageBps?: number
	/** Optional recipient. When set, executable transaction calldata is included. */
	recipient?: HexString
}

/**
 * Normalized quote returned by every Uniswap protocol adapter.
 */
export interface UniswapQuote {
	/** Protocol that produced this quote. */
	protocol: UniswapProtocol
	/** Quote direction used to calculate the amounts. */
	tradeType: UniswapTradeType
	/** Numeric EVM chain ID. */
	chainId: number
	/** Token the user pays with. */
	tokenIn: UniswapQuoteToken
	/** Token the user receives. */
	tokenOut: UniswapQuoteToken
	/** Input amount required for this quote. */
	amountIn: bigint
	/** Output amount returned by this quote. */
	amountOut: bigint
	/** Winning fee tier for concentrated-liquidity protocols. */
	fee?: number
	/** Token path used by the quote. */
	route: {
		tokens: HexString[]
	}
	/** Executable transactions, included only when `recipient` is supplied. */
	transactions?: Transaction[]
}

/**
 * Quote result containing the best quote and all successful protocol candidates.
 */
export interface QuoteUniswapResult {
	/** Best quote by highest output for exact input, or lowest input for exact output. */
	bestQuote: UniswapQuote | null
	/** All successful protocol quotes considered by the SDK. */
	quotes: UniswapQuote[]
}

interface ProtocolQuoteOptions {
	selectedProtocol?: UniswapProtocol
	generateCalldata?: boolean
	recipient?: HexString
}

export interface UniswapQuoteAdapter {
	findBestProtocolWithAmountIn(
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
		options?: ProtocolQuoteOptions,
	): Promise<{
		protocol: UniswapProtocol | null
		amountOut: bigint
		fee?: number
		transactions?: Transaction[]
	}>
	findBestProtocolWithAmountOut(
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
		options?: ProtocolQuoteOptions,
	): Promise<{
		protocol: UniswapProtocol | null
		amountIn: bigint
		fee?: number
		transactions?: Transaction[]
	}>
	createV2SwapCalldataExactIn(
		path: HexString[],
		amountIn: bigint,
		amountOutMinimum: bigint,
		recipient: HexString,
		evmChainID: string,
	): Transaction[]
	createV2SwapCalldataExactOut(
		path: HexString[],
		amountOut: bigint,
		amountInMax: bigint,
		recipient: HexString,
		evmChainID: string,
	): Transaction[]
	createV3SwapCalldataExactIn(
		path: HexString[],
		amountIn: bigint,
		amountOutMinimum: bigint,
		fees: number[],
		recipient: HexString,
		evmChainID: string,
	): Transaction[]
	createV3SwapCalldataExactOut(
		path: HexString[],
		amountOut: bigint,
		amountInMax: bigint,
		fees: number[],
		recipient: HexString,
		evmChainID: string,
	): Transaction[]
	createV4SwapCalldataExactIn(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountIn: bigint,
		amountOutMinimum: bigint,
		fee: number,
		evmChainID: string,
	): Transaction[]
	createV4SwapCalldataExactOut(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		fee: number,
		evmChainID: string,
	): Transaction[]
}

interface QuoteOptions {
	client?: PublicClient
}

export class UniswapQuoteEngine {
	constructor(
		private readonly adapter: UniswapQuoteAdapter,
		private readonly chainConfigService = new ChainConfigService(),
	) {}

	async quote(params: QuoteUniswapParams, options: QuoteOptions = {}): Promise<QuoteUniswapResult> {
		this.validateQuoteParams(params)

		const protocols = this.getQuoteProtocols(params.protocols)
		const { chainId, stateMachineId: evmChainID } = normalizeEvmChainId(params.chainId)
		const client = options.client ?? this.resolveClient(params.chainId)

		const quotes: UniswapQuote[] = []
		for (const protocol of protocols) {
			const quote =
				params.tradeType === "EXACT_INPUT"
					? await this.quoteExactInputForProtocol(params, client, protocol, chainId, evmChainID)
					: await this.quoteExactOutputForProtocol(params, client, protocol, chainId, evmChainID)
			if (quote) quotes.push(quote)
		}

		return { bestQuote: this.selectBestQuote(quotes, params.tradeType), quotes }
	}

	private resolveClient(chainId: number | string): PublicClient {
		const { stateMachineId: evmChainID } = normalizeEvmChainId(chainId)
		const rpcUrl = this.chainConfigService.getRpcUrl(evmChainID)
		if (!rpcUrl) {
			throw new Error(`No RPC URL configured for chain ${evmChainID}`)
		}
		return createPublicClient({ transport: http(rpcUrl) })
	}

	private getQuoteProtocols(protocols?: UniswapProtocol[]): UniswapProtocol[] {
		return protocols?.length ? protocols : ["v2", "v3", "v4"]
	}

	private validateQuoteParams(params: QuoteUniswapParams): void {
		if (params.tradeType === "EXACT_INPUT" && params.amountIn === undefined) {
			throw new Error("amountIn is required for EXACT_INPUT quotes")
		}
		if (params.tradeType === "EXACT_OUTPUT" && params.amountOut === undefined) {
			throw new Error("amountOut is required for EXACT_OUTPUT quotes")
		}
		if (params.slippageBps !== undefined && (params.slippageBps < 0 || params.slippageBps > 10_000)) {
			throw new Error("slippageBps must be between 0 and 10000")
		}
	}

	private applySlippageToAmountOut(amountOut: bigint, slippageBps: number): bigint {
		return (amountOut * BigInt(10_000 - slippageBps)) / 10_000n
	}

	private applySlippageToAmountIn(amountIn: bigint, slippageBps: number): bigint {
		const numerator = amountIn * BigInt(10_000 + slippageBps)
		return (numerator + 9_999n) / 10_000n
	}

	private selectBestQuote(quotes: UniswapQuote[], tradeType: UniswapTradeType): UniswapQuote | null {
		if (quotes.length === 0) return null

		if (tradeType === "EXACT_INPUT") {
			return quotes.reduce((best, current) => {
				return current.amountOut > best.amountOut ? current : best
			})
		}

		return quotes.reduce((best, current) => {
			return current.amountIn < best.amountIn ? current : best
		})
	}

	private createExactInputTransactions(
		protocol: UniswapProtocol,
		params: QuoteUniswapParams,
		minAmountOut: bigint,
		fee: number | undefined,
		evmChainID: string,
	): Transaction[] {
		const amountIn = params.amountIn!
		const recipient = params.recipient!
		const path = [params.tokenIn.address, params.tokenOut.address]

		switch (protocol) {
			case "v2":
				return this.adapter.createV2SwapCalldataExactIn(path, amountIn, minAmountOut, recipient, evmChainID)
			case "v3":
				return this.adapter.createV3SwapCalldataExactIn(
					path,
					amountIn,
					minAmountOut,
					[fee!],
					recipient,
					evmChainID,
				)
			case "v4":
				return this.adapter.createV4SwapCalldataExactIn(
					params.tokenIn.address,
					params.tokenOut.address,
					amountIn,
					minAmountOut,
					fee!,
					evmChainID,
				)
		}
	}

	private createExactOutputTransactions(
		protocol: UniswapProtocol,
		params: QuoteUniswapParams,
		maxAmountIn: bigint,
		fee: number | undefined,
		evmChainID: string,
	): Transaction[] {
		const amountOut = params.amountOut!
		const recipient = params.recipient!
		const path = [params.tokenIn.address, params.tokenOut.address]

		switch (protocol) {
			case "v2":
				return this.adapter.createV2SwapCalldataExactOut(path, amountOut, maxAmountIn, recipient, evmChainID)
			case "v3":
				return this.adapter.createV3SwapCalldataExactOut(
					path,
					amountOut,
					maxAmountIn,
					[fee!],
					recipient,
					evmChainID,
				)
			case "v4":
				return this.adapter.createV4SwapCalldataExactOut(
					params.tokenIn.address,
					params.tokenOut.address,
					amountOut,
					maxAmountIn,
					fee!,
					evmChainID,
				)
		}
	}

	private async quoteExactInputForProtocol(
		params: QuoteUniswapParams,
		client: PublicClient,
		protocol: UniswapProtocol,
		chainId: number,
		evmChainID: string,
	): Promise<UniswapQuote | null> {
		const generateCalldata = !!params.recipient
		const result = await this.adapter.findBestProtocolWithAmountIn(
			client,
			params.tokenIn.address,
			params.tokenOut.address,
			params.amountIn!,
			evmChainID,
			{
				selectedProtocol: protocol,
				generateCalldata,
				recipient: params.recipient,
			},
		)
		if (result.protocol === null || result.amountOut === 0n) return null

		let transactions = result.transactions
		const slippageBps = params.slippageBps ?? 0
		if (generateCalldata && slippageBps > 0) {
			const minAmountOut = this.applySlippageToAmountOut(result.amountOut, slippageBps)
			transactions = this.createExactInputTransactions(protocol, params, minAmountOut, result.fee, evmChainID)
		}

		return {
			protocol,
			tradeType: params.tradeType,
			chainId,
			tokenIn: params.tokenIn,
			tokenOut: params.tokenOut,
			amountIn: params.amountIn!,
			amountOut: result.amountOut,
			fee: result.fee,
			route: { tokens: [params.tokenIn.address, params.tokenOut.address] },
			transactions,
		}
	}

	private async quoteExactOutputForProtocol(
		params: QuoteUniswapParams,
		client: PublicClient,
		protocol: UniswapProtocol,
		chainId: number,
		evmChainID: string,
	): Promise<UniswapQuote | null> {
		const generateCalldata = !!params.recipient
		const result = await this.adapter.findBestProtocolWithAmountOut(
			client,
			params.tokenIn.address,
			params.tokenOut.address,
			params.amountOut!,
			evmChainID,
			{
				selectedProtocol: protocol,
				generateCalldata,
				recipient: params.recipient,
			},
		)
		if (result.protocol === null || result.amountIn === maxUint256) return null

		let transactions = result.transactions
		const slippageBps = params.slippageBps ?? 0
		if (generateCalldata && slippageBps > 0) {
			const maxAmountIn = this.applySlippageToAmountIn(result.amountIn, slippageBps)
			transactions = this.createExactOutputTransactions(protocol, params, maxAmountIn, result.fee, evmChainID)
		}

		return {
			protocol,
			tradeType: params.tradeType,
			chainId,
			tokenIn: params.tokenIn,
			tokenOut: params.tokenOut,
			amountIn: result.amountIn,
			amountOut: params.amountOut!,
			fee: result.fee,
			route: { tokens: [params.tokenIn.address, params.tokenOut.address] },
			transactions,
		}
	}
}
