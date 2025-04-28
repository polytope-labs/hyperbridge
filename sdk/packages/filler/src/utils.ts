import { ADDRESS_ZERO } from "hyperbridge-sdk"

export async function fetchTokenUsdPriceOnchain(address: string, decimals: number): Promise<bigint> {
	if (address == ADDRESS_ZERO) {
		return BigInt(10 ** 18)
	}

	try {
		const response = await fetch(
			`https://api.coingecko.com/api/v3/simple/token_price/ethereum?contract_addresses=${address}&vs_currencies=usd`,
		)
		const data = await response.json()

		if (!data[address.toLowerCase()]?.usd) {
			throw new Error(`Price not found for token address: ${address}`)
		}

		return BigInt(Math.floor(data[address.toLowerCase()].usd * 10 ** decimals))
	} catch (error) {
		console.error("Error fetching token price:", error)
		throw error
	}
}
