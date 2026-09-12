import { ERC6160Ext20Abi__factory, EthereumHostAbi__factory } from "@/configs/src/types/contracts"
import { CHAINS_BY_ISMP_HOST } from "@/constants"

export interface FeeTokenInfo {
	/** Address of the host's fee token */
	address: string
	/** Decimals the fee token's amounts are denominated in */
	decimals: number
}

// Host governance can replace the fee token. Cache only the last indexed block per chain.
const feeTokenCache = new Map<string, { blockHash: string; info: FeeTokenInfo }>()

/**
 * Returns the address and decimals of a chain's ISMP host fee token, cached per chain and block hash.
 * @param chain The chain's state machine identifier (e.g., "EVM-56")
 * @throws Error if no host address is configured for the chain or a contract call fails
 */
export async function getHostFeeToken(chain: string, blockHash: string): Promise<FeeTokenInfo> {
	const cached = feeTokenCache.get(chain)
	if (cached?.blockHash === blockHash) return cached.info

	const hostAddress = CHAINS_BY_ISMP_HOST[chain]
	if (!hostAddress) {
		throw new Error(`No ISMP host address configured for chain: ${chain}`)
	}

	const address = (
		await EthereumHostAbi__factory.connect(hostAddress, api).feeToken({ blockTag: blockHash })
	).toLowerCase()
	const decimals = await ERC6160Ext20Abi__factory.connect(address, api).decimals({ blockTag: blockHash })

	const feeToken = { address, decimals }
	feeTokenCache.set(chain, { blockHash, info: feeToken })
	return feeToken
}
