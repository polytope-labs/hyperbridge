export async function fetchTokenUsdPriceOnchain(address: string): Promise<bigint> {
	try {
		const response = await fetch(
			`https://api.coingecko.com/api/v3/simple/token_price/ethereum?contract_addresses=${address}&vs_currencies=usd`,
		)
		const data = await response.json()

		if (!data[address.toLowerCase()]?.usd) {
			throw new Error(`Price not found for token address: ${address}`)
		}

		return BigInt(Math.floor(data[address.toLowerCase()].usd))
	} catch (error) {
		console.log("Testnet token price not found, returning 1")
		return BigInt(1)
	}
}
