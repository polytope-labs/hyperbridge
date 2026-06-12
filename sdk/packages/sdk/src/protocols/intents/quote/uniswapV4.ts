import { decodeFunctionResult, encodeFunctionData, getAddress, type PublicClient, zeroAddress } from "viem"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import type { ConfiguredAssetSymbol, UniswapV4PoolConfigData } from "@/configs/chain"
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

type GatewayParamsObject = { protocolFeeBps?: bigint | number | string }
type GatewayParams = GatewayParamsObject | readonly unknown[]

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
	constructor(private readonly chainConfigService: ChainConfigService) {}

	async quote(
		params: QuoteIntentParams,
		source: IntentQuoteChainContext,
		destination: IntentQuoteChainContext,
	): Promise<QuoteIntentResult> {
		this.validateQuoteParams(params)

		const protocolFeeBps = await this.readProtocolFeeBps(source.client, source.stateMachineId)
		const poolConfig = this.resolvePoolConfig(params, source.stateMachineId, destination.stateMachineId)

		return params.amountIn !== undefined
			? this.quoteExactInput({ params, client: destination.client, protocolFeeBps, poolConfig })
			: this.quoteExactOutput({ params, client: destination.client, protocolFeeBps, poolConfig })
	}

	private validateQuoteParams(params: QuoteIntentParams): void {
		const hasAmountIn = params.amountIn !== undefined
		const hasAmountOut = params.amountOut !== undefined
		if (hasAmountIn === hasAmountOut) throw new Error("Provide exactly one of amountIn or amountOut")
		if (hasAmountIn && params.amountIn! <= 0n) throw new Error("amountIn must be greater than zero")
		if (hasAmountOut && params.amountOut! <= 0n) throw new Error("amountOut must be greater than zero")
		if (params.tokenIn.address.toLowerCase() === params.tokenOut.address.toLowerCase()) {
			throw new Error("tokenIn and tokenOut cannot be the same")
		}
	}

	private async readProtocolFeeBps(client: PublicClient, chain: string): Promise<bigint> {
		const gatewayAddress = this.chainConfigService.getIntentGatewayAddress(chain)
		if (!gatewayAddress || gatewayAddress === "0x" || gatewayAddress === zeroAddress) {
			throw new Error(`IntentGatewayV2 is not configured for chain ${chain}`)
		}

		const gatewayParams = (await client.readContract({
			address: gatewayAddress,
			abi: IntentGatewayV2.ABI,
			functionName: "params",
		})) as GatewayParams

		if (isGatewayParamsTuple(gatewayParams)) return BigInt(gatewayParams[4] as bigint | number | string)
		return BigInt(gatewayParams.protocolFeeBps ?? 0)
	}

	private resolvePoolConfig(params: QuoteIntentParams, source: string, destination: string): ResolvedPoolConfig {
		const override = params.uniswapV4?.poolKey
		if (override) {
			const poolKey = normalizePoolKey(override)
			const tokenInForQuote = getAddress(override.currencyIn ?? params.tokenIn.address) as HexString
			if (tokenInForQuote !== poolKey.currency0 && tokenInForQuote !== poolKey.currency1) {
				throw new Error(
					`Input currency ${tokenInForQuote} is not part of the override pool ` +
						`(${poolKey.currency0}, ${poolKey.currency1}). For cross-chain quotes pass ` +
						`uniswapV4.poolKey.currencyIn with the destination-side input currency address.`,
				)
			}

			return {
				poolKey,
				quoterAddress: this.resolveQuoterAddress(destination, override.quoterAddress),
				tokenInForQuote,
			}
		}

		const poolConfig = this.resolveConfiguredPool(params, destination, source === destination)
		if (poolConfig) return poolConfig

		throw new UnsupportedIntentQuotePairError({
			source,
			destination,
			tokenIn: params.tokenIn,
			tokenOut: params.tokenOut,
		})
	}

	private resolveConfiguredPool(
		params: QuoteIntentParams,
		chain: string,
		sameChain: boolean,
	): ResolvedPoolConfig | null {
		for (const pool of this.chainConfigService.getUniswapV4PoolConfigs(chain)) {
			const resolvedPool = this.resolveConfiguredPoolTokens(chain, pool)
			if (!resolvedPool) continue

			// tokenIn lives on the source chain, so its address is only meaningful
			// against destination-chain config when source === destination.
			// tokenOut is always a destination-side address.
			const tokenInForQuote = sameChain
				? matchPoolTokenByAddress(params.tokenIn, resolvedPool)
				: matchPoolTokenBySymbol(params.tokenIn, resolvedPool)
			const tokenOutForQuote = matchPoolTokenByAddress(params.tokenOut, resolvedPool)
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
		const amountIn = args.params.amountIn!
		const amountOut = await this.readV4QuoteExactInput(args.client, args.poolConfig, amountIn)

		return {
			strategy: "uniswap_v4",
			tradeType: "EXACT_INPUT",
			amountIn,
			amountOut,
			quoteMetadata: {
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
		const amountOut = args.params.amountOut!
		const amountIn = await this.readV4QuoteExactOutput(args.client, args.poolConfig, amountOut)

		return {
			strategy: "uniswap_v4",
			tradeType: "EXACT_OUTPUT",
			amountIn,
			amountOut,
			quoteMetadata: {
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

function matchPoolTokenByAddress(
	token: IntentQuoteToken,
	poolTokens: readonly [ResolvedConfiguredPoolToken, ResolvedConfiguredPoolToken],
): ResolvedConfiguredPoolToken | null {
	const tokenAddress = token.address.toLowerCase()
	return poolTokens.find((poolToken) => poolToken.address.toLowerCase() === tokenAddress) ?? null
}

function matchPoolTokenBySymbol(
	token: IntentQuoteToken,
	poolTokens: readonly [ResolvedConfiguredPoolToken, ResolvedConfiguredPoolToken],
): ResolvedConfiguredPoolToken | null {
	const tokenSymbol = token.symbol?.toUpperCase()
	if (!tokenSymbol) return null
	return poolTokens.find((poolToken) => poolToken.symbol.toUpperCase() === tokenSymbol) ?? null
}

function getZeroForOne(tokenIn: HexString, poolKey: UniswapV4PoolKey): boolean {
	return getAddress(tokenIn).toLowerCase() === getAddress(poolKey.currency0).toLowerCase()
}

function isGatewayParamsTuple(value: GatewayParams): value is readonly unknown[] {
	return Array.isArray(value)
}
