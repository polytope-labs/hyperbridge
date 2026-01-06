import { EthereumResult, EthereumLog } from "@subql/types-ethereum"
import { bytesToHex, hexToBytes, pad } from "viem"
import type { Hex } from "viem"

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

/**
 * bytes20ToBytes32 converts a bytes20 address to bytes32 format by padding with zeros
 * Ensures all addresses are stored uniformly as 32 bytes
 */
export function bytes20ToBytes32(bytes20: string): string {
	// If already 32 bytes (66 chars with 0x), return as-is
	if (bytes20.length === 66) {
		return bytes20
	}

	// If 20 bytes (42 chars with 0x), pad to 32 bytes
	if (bytes20.length === 42) {
		return pad(bytes20 as Hex, { size: 32 }) as string
	}

	// If it's already 32 bytes but without proper format, ensure it's padded
	const cleaned = bytes20.startsWith("0x") ? bytes20 : `0x${bytes20}`
	return pad(cleaned as Hex, { size: 32 }).toLowerCase() as Hex
}

export function bytes32ToBytes20(bytes32: string): string {
	if (bytes32.length === 42) {
		return bytes32
	}

	const bytes = hexToBytes(bytes32 as Hex)
	const addressBytes = bytes.slice(12)
	return bytesToHex(addressBytes).toLowerCase() as Hex
}
