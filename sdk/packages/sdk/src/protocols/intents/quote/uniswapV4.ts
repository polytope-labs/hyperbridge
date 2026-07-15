import {
	createPublicClient,
	decodeFunctionResult,
	encodeFunctionData,
	getAddress,
	http,
	type PublicClient,
	zeroAddress,
} from "viem"
import { base } from "viem/chains"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import { Chains, type ConfiguredAssetSymbol, type UniswapV4PoolConfigData } from "@/configs/chain"
import type { HexString } from "@/types"
import {
	type IntentQuoteChainContext,
	type IntentQuoteStrategyHandler,
	type IntentQuoteToken,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedIntentQuotePairError,
	type UniswapV4PoolKey,
} from "./types"
import { deductProtocolFee, grossUpForProtocolFee, readProtocolFeeBps, validateQuoteParams } from "./shared"
export { deductProtocolFee, grossUpForProtocolFee } from "./shared"

export const UNISWAP_INTENT_QUOTE_CHAIN = Chains.BASE_MAINNET

interface ResolvedPoolConfig {
	poolKey: UniswapV4PoolKey
	quoterAddress: HexString
	tokenInForQuote: HexString
}

interface ResolvedConfiguredPoolToken {
	symbol: ConfiguredAssetSymbol
	address: HexString
}

export class UniswapV4IntentQuoteStrategy implements IntentQuoteStrategyHandler {
	private baseQuoteClient?: PublicClient

	constructor(private readonly chainConfigService: ChainConfigService) {}

	async quote(
		params: QuoteIntentParams,
		source: IntentQuoteChainContext,
		destination: IntentQuoteChainContext,
	): Promise<QuoteIntentResult> {
		validateQuoteParams(params)

		const protocolFeeBps = await readProtocolFeeBps(this.chainConfigService, source)
		const quoteClient = this.resolveQuoteClient(source, destination)
		const poolConfig = this.resolvePoolConfig(params, source.stateMachineId, UNISWAP_INTENT_QUOTE_CHAIN)

		return params.amountIn !== undefined
			? this.quoteExactInput({ params, client: quoteClient, protocolFeeBps, poolConfig })
			: this.quoteExactOutput({ params, client: quoteClient, protocolFeeBps, poolConfig })
	}

	private resolveQuoteClient(source: IntentQuoteChainContext, destination: IntentQuoteChainContext): PublicClient {
		if (source.stateMachineId === UNISWAP_INTENT_QUOTE_CHAIN) return source.client
		if (destination.stateMachineId === UNISWAP_INTENT_QUOTE_CHAIN) return destination.client
		if (this.baseQuoteClient) return this.baseQuoteClient

		const rpcUrl = this.chainConfigService.getRpcUrl(UNISWAP_INTENT_QUOTE_CHAIN)
		if (!rpcUrl) throw new Error(`RPC URL is not configured for ${UNISWAP_INTENT_QUOTE_CHAIN}`)

		const baseQuoteClient = createPublicClient({
			chain: base,
			transport: http(rpcUrl),
		}) as PublicClient
		this.baseQuoteClient = baseQuoteClient
		return baseQuoteClient
	}

	private resolvePoolConfig(params: QuoteIntentParams, source: string, destination: string): ResolvedPoolConfig {
		const override = params.uniswapV4?.poolKey
		if (override) {
			const poolKey = normalizePoolKey(override)
			const tokenInForQuote = getAddress(override.currencyIn ?? params.tokenIn.address) as HexString
			if (tokenInForQuote !== poolKey.currency0 && tokenInForQuote !== poolKey.currency1) {
				throw new Error(
					`Input currency ${tokenInForQuote} is not part of the override pool (${poolKey.currency0}, ${poolKey.currency1}). For cross-chain quotes pass uniswapV4.poolKey.currencyIn with the Base-side input currency address.`,
				)
			}

			return {
				poolKey,
				quoterAddress: this.resolveQuoterAddress(destination, override.quoterAddress),
				tokenInForQuote,
			}
		}

		const poolConfig = this.resolveConfiguredPool(params, destination)
		if (poolConfig) return poolConfig

		throw new UnsupportedIntentQuotePairError({
			source,
			destination,
			tokenIn: params.tokenIn,
			tokenOut: params.tokenOut,
		})
	}

	private resolveConfiguredPool(params: QuoteIntentParams, chain: string): ResolvedPoolConfig | null {
		for (const pool of this.chainConfigService.getUniswapV4PoolConfigs(chain)) {
			const resolvedPool = this.resolveConfiguredPoolTokens(chain, pool)
			if (!resolvedPool) continue

			const tokenInForQuote = matchPoolToken(params.tokenIn, resolvedPool)
			const tokenOutForQuote = matchPoolToken(params.tokenOut, resolvedPool)
			if (!tokenInForQuote || !tokenOutForQuote || tokenInForQuote.symbol === tokenOutForQuote.symbol) continue

			const { currency0, currency1 } = sortCurrencies(resolvedPool[0].address, resolvedPool[1].address)
			return {
				poolKey: {
					currency0,
					currency1,
					fee: pool.fee,
					tickSpacing: pool.tickSpacing,
					hooks: getAddress(pool.hooks ?? zeroAddress) as HexString,
				},
				quoterAddress: this.resolveQuoterAddress(chain),
				tokenInForQuote: tokenInForQuote.address,
			}
		}

		return null
	}

	private resolveQuoterAddress(chain: string, override?: HexString): HexString {
		const address = override ?? this.chainConfigService.getUniswapV4QuoterAddress(chain)
		if (!address || address === "0x" || address === zeroAddress) {
			throw new Error(`Uniswap V4 quoter is not configured for chain ${chain}`)
		}
		return getAddress(address) as HexString
	}

	private resolveConfiguredPoolTokens(
		chain: string,
		pool: UniswapV4PoolConfigData,
	): readonly [ResolvedConfiguredPoolToken, ResolvedConfiguredPoolToken] | null {
		const first = this.resolveConfiguredPoolToken(chain, pool.tokens[0])
		const second = this.resolveConfiguredPoolToken(chain, pool.tokens[1])
		return first && second ? [first, second] : null
	}

	private resolveConfiguredPoolToken(
		chain: string,
		symbol: ConfiguredAssetSymbol,
	): ResolvedConfiguredPoolToken | null {
		const address = this.chainConfigService.getAssetAddress(chain, symbol)
		if (!address || address === "0x") return null
		return { symbol, address: getAddress(address) as HexString }
	}

	private async quoteExactInput(args: {
		params: QuoteIntentParams
		client: PublicClient
		protocolFeeBps: bigint
		poolConfig: ResolvedPoolConfig
	}): Promise<QuoteIntentResult> {
		const amountIn = args.params.amountIn
		if (amountIn === undefined) throw new Error("amountIn is required for an exact-input quote")
		// The gateway deducts its protocol fee from order inputs, so only the
		// reduced amount reaches the swap. Quote against that net amount.
		const swapAmountIn = deductProtocolFee(amountIn, args.protocolFeeBps)
		const amountOut = await this.readV4QuoteExactInput(args.client, args.poolConfig, swapAmountIn)

		return {
			strategy: "uniswap_v4",
			tradeType: "EXACT_INPUT",
			amountIn,
			amountOut,
			quoteMetadata: {
				quoteChain: UNISWAP_INTENT_QUOTE_CHAIN,
				poolKey: args.poolConfig.poolKey,
				quoterAddress: args.poolConfig.quoterAddress,
				protocolFeeBps: args.protocolFeeBps,
			},
		}
	}

	private async quoteExactOutput(args: {
		params: QuoteIntentParams
		client: PublicClient
		protocolFeeBps: bigint
		poolConfig: ResolvedPoolConfig
	}): Promise<QuoteIntentResult> {
		const amountOut = args.params.amountOut
		if (amountOut === undefined) throw new Error("amountOut is required for an exact-output quote")
		// The quoter returns the swap input needed for `amountOut`; that is the
		// net amount after the gateway's protocol fee, so gross it back up to the
		// order input the caller must supply.
		const swapAmountIn = await this.readV4QuoteExactOutput(args.client, args.poolConfig, amountOut)
		const amountIn = grossUpForProtocolFee(swapAmountIn, args.protocolFeeBps)

		return {
			strategy: "uniswap_v4",
			tradeType: "EXACT_OUTPUT",
			amountIn,
			amountOut,
			quoteMetadata: {
				quoteChain: UNISWAP_INTENT_QUOTE_CHAIN,
				poolKey: args.poolConfig.poolKey,
				quoterAddress: args.poolConfig.quoterAddress,
				protocolFeeBps: args.protocolFeeBps,
			},
		}
	}

	private async readV4QuoteExactInput(
		client: PublicClient,
		poolConfig: ResolvedPoolConfig,
		amountIn: bigint,
	): Promise<bigint> {
		const data = encodeFunctionData({
			abi: UNISWAP_V4_QUOTER_ABI,
			functionName: "quoteExactInputSingle",
			args: [
				{
					poolKey: poolConfig.poolKey,
					zeroForOne: getZeroForOne(poolConfig.tokenInForQuote, poolConfig.poolKey),
					exactAmount: amountIn,
					hookData: "0x",
				},
			],
		})
		const response = await client.call({ to: poolConfig.quoterAddress, data })
		if (!response.data || response.data === "0x") {
			throw new Error(`Uniswap V4 quoter at ${poolConfig.quoterAddress} returned no data`)
		}

		const [amountOut] = decodeFunctionResult({
			abi: UNISWAP_V4_QUOTER_ABI,
			functionName: "quoteExactInputSingle",
			data: response.data,
		})

		return amountOut
	}

	private async readV4QuoteExactOutput(
		client: PublicClient,
		poolConfig: ResolvedPoolConfig,
		amountOut: bigint,
	): Promise<bigint> {
		const data = encodeFunctionData({
			abi: UNISWAP_V4_QUOTER_ABI,
			functionName: "quoteExactOutputSingle",
			args: [
				{
					poolKey: poolConfig.poolKey,
					zeroForOne: getZeroForOne(poolConfig.tokenInForQuote, poolConfig.poolKey),
					exactAmount: amountOut,
					hookData: "0x",
				},
			],
		})
		const response = await client.call({ to: poolConfig.quoterAddress, data })
		if (!response.data || response.data === "0x") {
			throw new Error(`Uniswap V4 quoter at ${poolConfig.quoterAddress} returned no data`)
		}

		const [amountIn] = decodeFunctionResult({
			abi: UNISWAP_V4_QUOTER_ABI,
			functionName: "quoteExactOutputSingle",
			data: response.data,
		})

		return amountIn
	}
}

function normalizePoolKey(poolKey: UniswapV4PoolKey): UniswapV4PoolKey {
	return {
		currency0: getAddress(poolKey.currency0) as HexString,
		currency1: getAddress(poolKey.currency1) as HexString,
		fee: poolKey.fee,
		tickSpacing: poolKey.tickSpacing,
		hooks: getAddress(poolKey.hooks) as HexString,
	}
}

function sortCurrencies(tokenIn: HexString, tokenOut: HexString): Pick<UniswapV4PoolKey, "currency0" | "currency1"> {
	const input = getAddress(tokenIn)
	const output = getAddress(tokenOut)
	return BigInt(input) < BigInt(output)
		? { currency0: input as HexString, currency1: output as HexString }
		: { currency0: output as HexString, currency1: input as HexString }
}

function matchPoolToken(
	token: IntentQuoteToken,
	poolTokens: readonly [ResolvedConfiguredPoolToken, ResolvedConfiguredPoolToken],
): ResolvedConfiguredPoolToken | null {
	const tokenAddress = token.address.toLowerCase()
	const addressMatch = poolTokens.find((poolToken) => poolToken.address.toLowerCase() === tokenAddress)
	if (addressMatch) return addressMatch

	const tokenSymbol = token.symbol?.toUpperCase()
	if (!tokenSymbol) return null
	return poolTokens.find((poolToken) => poolToken.symbol.toUpperCase() === tokenSymbol) ?? null
}

function getZeroForOne(tokenIn: HexString, poolKey: UniswapV4PoolKey): boolean {
	return getAddress(tokenIn).toLowerCase() === getAddress(poolKey.currency0).toLowerCase()
}
