import { EthereumResult, EthereumLog } from "@subql/types-ethereum"

import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import type { PriceResponse } from "./price.helpers"
import { TokenPriceService } from "@/services/token-price.service"
import PriceHelper from "./price.helpers"

// ERC20 Transfer event signature: Transfer(address indexed from, address indexed to, uint256 value)
const ERC20_TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"

/**
 * isERC20TransferEvent checks if the topic includes Transfer(address, address, uint256).
 * @param log EthereumLog<EthereumResult>
 * @returns boolean
 */
export const isERC20TransferEvent = (log: EthereumLog<EthereumResult>): boolean => {
	return (
		log.topics && log.topics.length >= 3 && log.topics[0] === ERC20_TRANSFER_TOPIC // Check exact match for topics[0]
	)
}

type GetPriceFromEthereumLogResponse = Promise<PriceResponse & { symbol: string; decimals: number }>

/**
 * getPriceDataFromEthereumLog retrieves price data from an Ethereum log.
 * @param log EthereumLog<EthereumResult>
 * @returns Promise<GetPriceDataFromEthereumLogResponse>
 */
export const getPriceDataFromEthereumLog = async (
	address: string,
	amount: bigint,
	currentTimestamp?: bigint,
): GetPriceFromEthereumLogResponse => {
	const contract = ERC6160Ext20Abi__factory.connect(address.toLowerCase(), api)

	const symbol = await contract.symbol()
	const decimals = await contract.decimals()

	const price = await TokenPriceService.getPrice(symbol, currentTimestamp)
	const { amountValueInUSD, priceInUSD } = PriceHelper.getAmountValueInUSD(amount, decimals, price)

	return {
		amountValueInUSD,
		priceInUSD,
		symbol,
		decimals,
	}
}

/**
 * extractAddressFromTopic converts a 32-byte indexed topic into a 20-byte EVM address
 */
export function extractAddressFromTopic(topic: string): string {
	if (topic.startsWith("0x") && topic.length === 66) {
		return ("0x" + topic.slice(26)).toLowerCase()
	}
	return topic.toLowerCase()
}
