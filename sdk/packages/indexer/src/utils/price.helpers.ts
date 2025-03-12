import { CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES } from "@/addresses/chainlink-price-feeds.addresses"
import { ITokenPriceFeedDetails } from "@/constants"
import { ChainLinkAggregatorV3Abi__factory } from "@/configs/src/types/contracts"

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
	static async getTokenPriceInUsd(priceFeedDetails: ITokenPriceFeedDetails): Promise<bigint> {
		const priceFeedContract = ChainLinkAggregatorV3Abi__factory.connect(priceFeedDetails.chain_link_price_feed, api)

		const roundData = await priceFeedContract.latestRoundData()
		const decimals = await priceFeedContract.decimals()
		let exponent = 18 - decimals

		// Ensure we convert to the standard 18 decimals used by erc20.
		return roundData.answer.toBigInt() * BigInt(10 ** exponent)
	}
}
