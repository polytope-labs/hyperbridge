import { CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES } from "@/addresses/chainlink-price-feeds.addresses"
import { ITokenPriceFeedDetails } from "@/constants"
import { ChainLinkAggregatorV3Abi__factory } from "@/configs/src/types/contracts"
import { ethers } from "ethers"
import { UNISWAP_ADDRESSES } from "@/addresses/uniswap.addresses"
import uniswapV2Abi from "@/configs/abis/UniswapV2.abi.json"
import uniswapV3FactoryAbi from "@/configs/abis/UniswapV3Factory.abi.json"
import uniswapV3PoolAbi from "@/configs/abis/UniswapV3Pool.abi.json"
import uniswapV3QuoterV2Abi from "@/configs/abis/UniswapV3QuoterV2.abi.json"
import uniswapV4QuoterAbi from "@/configs/abis/UniswapV4Quoter.abi.json"
import Decimal from "decimal.js"
import type { Hex } from "viem"

export default class PriceHelper {
	static async getNativeCurrencyPrice(stateMachineId: string): Promise<bigint> {
		const priceFeedAddress = CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES[stateMachineId]

		if (!priceFeedAddress) {
			throw new Error(`Price feed address not found for state machine id: ${stateMachineId}`)
		}

		const priceFeedContract = ChainLinkAggregatorV3Abi__factory.connect(priceFeedAddress, api)

		const roundData = await priceFeedContract.latestRoundData()
		const decimals = await priceFeedContract.decimals()
		let exponent = 18 - decimals

		// Ensure we convert to the standard 18 decimals used by erc20.
		return roundData.answer.toBigInt() * BigInt(10 ** exponent)
	}

	/**
	 * Get the current price IN USD for an ERC20 token given it's contract address
	 */
	static async getTokenPriceInUsdChainlink(priceFeedDetails: ITokenPriceFeedDetails): Promise<bigint> {
		const priceFeedContract = ChainLinkAggregatorV3Abi__factory.connect(priceFeedDetails.chain_link_price_feed, api)

		const roundData = await priceFeedContract.latestRoundData()
		const decimals = await priceFeedContract.decimals()
		let exponent = 18 - decimals

		// Ensure we convert to the standard 18 decimals used by erc20.
		return roundData.answer.toBigInt() * BigInt(10 ** exponent)
	}

	static async getTokenPriceInUSDUniswap(
		tokenAddress: string,
		amount: bigint,
		decimals: number,
	): Promise<{
		priceInUSD: string
		amountValueInUSD: string
	}> {
		try {
			const isNativeToken = tokenAddress.endsWith("0000000000000000000000000000000000000000")

			let priceInUSD: string
			if (isNativeToken) {
				const nativePrice = await this.getNativeCurrencyPrice(chainId)
				priceInUSD = new Decimal(nativePrice.toString()).dividedBy(new Decimal(10).pow(18)).toFixed(18)
			} else {
				const uniswapPrice = await this.getUniswapPrice(tokenAddress, decimals)
				priceInUSD = uniswapPrice.toFixed(18)
			}

			const amountValueInUSD = new Decimal(amount.toString())
				.dividedBy(new Decimal(10).pow(decimals))
				.times(new Decimal(priceInUSD))
				.toFixed(18)

			return {
				priceInUSD,
				amountValueInUSD,
			}
		} catch (error) {
			logger.error(`Error getting token price for ${tokenAddress}: ${error}`)
			return {
				priceInUSD: "0",
				amountValueInUSD: "0",
			}
		}
	}

	private static async getUniswapPrice(tokenAddress: string, decimals: number): Promise<Decimal> {
		try {
			const addresses = UNISWAP_ADDRESSES[`EVM-${chainId}`]
			const { V2_FACTORY, WETH, USDC, USDT, DAI, V4_QUOTER, V3_FACTORY, V3_QUOTER } = addresses

			const amountIn = BigInt(10 ** decimals)

			const quoteTokens = [USDC, USDT, DAI, WETH]

			// Try V2 first
			for (const quoteToken of quoteTokens) {
				const pairAddress = await this.getUniswapV2PairAddress(V2_FACTORY, tokenAddress, quoteToken)
				if (pairAddress) {
					try {
						const price = await this.getPriceFromPair(pairAddress, quoteToken)
						// If we got a price from WETH pair, convert to USD
						if (quoteToken === WETH) {
							const wethUsdcPair = await this.getUniswapV2PairAddress(V2_FACTORY, WETH, USDC)
							if (wethUsdcPair) {
								const wethUsdPrice = await this.getPriceFromPair(wethUsdcPair, USDC)
								return price.times(wethUsdPrice)
							}
						}
						return price
					} catch (error) {
						logger.error(`Error getting price from V2 pair ${pairAddress}: ${error}`)
						continue
					}
				}
			}

			// If V2 fails, try V3

			if (V3_FACTORY && V3_QUOTER) {
				// Out of three tiers, we return the first one that works
				const fees = [500, 3000, 10000] // 0.05%, 0.3%, 1%

				for (const quoteToken of quoteTokens) {
					for (const fee of fees) {
						try {
							const factory = new ethers.Contract(V3_FACTORY, uniswapV3FactoryAbi, api)
							const pool = await factory.getPool(tokenAddress, quoteToken, fee)

							if (pool && pool !== "0x0000000000000000000000000000000000000000") {
								const poolContract = new ethers.Contract(pool, uniswapV3PoolAbi, api)
								const liquidity = await poolContract.liquidity()

								if (liquidity > BigInt(0)) {
									const quoter = new ethers.Contract(V3_QUOTER, uniswapV3QuoterV2Abi, api)
									const quoteResult = await quoter.quoteExactInputSingle({
										tokenIn: tokenAddress,
										tokenOut: quoteToken,
										fee: fee,
										amount: amountIn,
										sqrtPriceLimitX96: BigInt(0),
									})

									const [amountOut] = quoteResult
									let price = new Decimal(amountOut.toString()).dividedBy(new Decimal(10 ** decimals))

									// If we got a price from WETH pair, convert to USD
									if (quoteToken === WETH) {
										const wethUsdcPool = await factory.getPool(WETH, USDC, 500) // Use 0.05% fee tier for stable pairs

										if (
											wethUsdcPool &&
											wethUsdcPool !== "0x0000000000000000000000000000000000000000"
										) {
											const wethQuoteResult = await quoter.quoteExactInputSingle({
												tokenIn: WETH,
												tokenOut: USDC,
												fee: 500,
												amount: BigInt(10 ** decimals),
												sqrtPriceLimitX96: BigInt(0),
											})

											const [wethAmountOut] = wethQuoteResult
											const wethUsdPrice = new Decimal(wethAmountOut.toString()).dividedBy(
												new Decimal(10 ** decimals),
											)
											price = price.times(wethUsdPrice)
										}
									}

									// Return the first valid price we find
									return price
								}
							}
						} catch (error) {
							logger.error(`Error getting price from V3 pool for fee ${fee}: ${error}`)
							continue
						}
					}
				}
			}

			// If V3 fails, try V4
			if (V4_QUOTER) {
				for (const quoteToken of quoteTokens) {
					for (const fee of [500, 3000, 10000]) {
						// Same fee tiers as V3
						try {
							const quoter = new ethers.Contract(V4_QUOTER, uniswapV4QuoterAbi, api)

							const [token0, token1] =
								tokenAddress.toLowerCase() < quoteToken.toLowerCase()
									? [tokenAddress, quoteToken]
									: [quoteToken, tokenAddress]

							const poolKey = {
								currency0: token0,
								currency1: token1,
								fee: fee,
								tickSpacing: fee === 500 ? 10 : fee === 3000 ? 60 : 200, // Tick spacing changes based on fee
								hooks: "0x0000000000000000000000000000000000000000" as Hex,
							}

							// Determine if token0 is the input token
							const zeroForOne = tokenAddress.toLowerCase() === token0.toLowerCase()

							const quoteParams = {
								poolKey,
								zeroForOne,
								exactAmount: amountIn,
								hookData: "0x" as Hex,
							}

							const quoteResult = await quoter.callStatic.quoteExactInputSingle(quoteParams)
							const amountOut = quoteResult.amountOut

							let price = new Decimal(amountOut.toString()).dividedBy(new Decimal(10 ** decimals))

							if (quoteToken === WETH) {
								const wethUsdcPoolKey = {
									currency0: WETH,
									currency1: USDC,
									fee: 500,
									tickSpacing: 10,
									hooks: "0x0000000000000000000000000000000000000000" as Hex,
								}

								const wethQuoteParams = {
									poolKey: wethUsdcPoolKey,
									zeroForOne: true,
									exactAmount: BigInt(10 ** decimals),
									hookData: "0x" as Hex,
								}

								const wethQuoteResult = await quoter.callStatic.quoteExactInputSingle(wethQuoteParams)
								const wethAmountOut = wethQuoteResult.amountOut
								const wethUsdPrice = new Decimal(wethAmountOut.toString()).dividedBy(
									new Decimal(10 ** decimals),
								)
								price = price.times(wethUsdPrice)
							}

							return price
						} catch (error) {
							logger.error(`Error getting price from V4 pool for fee ${fee}: ${error}`)
							continue
						}
					}
				}
			}

			throw new Error(`No Uniswap V2, V3, or V4 pair found for token ${tokenAddress}`)
		} catch (error) {
			logger.error(`Error getting Uniswap price for ${tokenAddress}: ${error}`)
			return new Decimal(1)
		}
	}

	private static async getUniswapV2PairAddress(
		factoryAddress: string,
		token0: string,
		token1: string,
	): Promise<string | null> {
		try {
			const factory = new ethers.Contract(factoryAddress, uniswapV2Abi.factory, api)

			const pairAddress = await factory.callStatic.getPair(token0, token1)
			logger.info(`Pair address for ${token0}/${token1}: ${pairAddress}`)
			return pairAddress === "0x0000000000000000000000000000000000000000" ? null : pairAddress
		} catch (error) {
			logger.error(`Error getting pair address for ${token0}/${token1}: ${error}`)
			return null
		}
	}

	private static async getPriceFromPair(pairAddress: string, quoteToken: string): Promise<Decimal> {
		try {
			const pair = new ethers.Contract(pairAddress, uniswapV2Abi.pair, api)

			const [token0, token1] = await Promise.all([pair.token0(), pair.token1()])

			const [reserve0, reserve1] = await pair.getReserves()

			// Calculate price based on reserves
			const reserveIn = token0.toLowerCase() === quoteToken.toLowerCase() ? reserve0 : reserve1
			const reserveOut = token0.toLowerCase() === quoteToken.toLowerCase() ? reserve1 : reserve0

			if (reserveIn.eq(0) || reserveOut.eq(0)) {
				throw new Error("Zero reserves")
			}

			return new Decimal(reserveIn.toString()).dividedBy(new Decimal(reserveOut.toString()))
		} catch (error) {
			logger.error(`Error getting price from pair ${pairAddress}: ${error}`)
			throw error
		}
	}
}
