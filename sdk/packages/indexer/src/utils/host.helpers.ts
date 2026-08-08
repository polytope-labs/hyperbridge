import { ERC6160Ext20Abi__factory, EthereumHostAbi__factory } from "@/configs/src/types/contracts"
import { CHAINS_BY_ISMP_HOST } from "@/constants"

export interface FeeTokenInfo {
	/** Address of the host's fee token */
	address: string
	/** Decimals the fee token's amounts are denominated in */
	decimals: number
}

// The host's fee token is static per chain, so one pair of contract calls per process suffices.
const feeTokenCache = new Map<string, FeeTokenInfo>()

/**
 * Returns the address and decimals of a chain's ISMP host fee token, cached per chain.
 * @param chain The chain's state machine identifier (e.g., "EVM-56")
 * @throws Error if no host address is configured for the chain or a contract call fails
 */
export async function getHostFeeToken(chain: string): Promise<FeeTokenInfo> {
	const cached = feeTokenCache.get(chain)
	if (cached) return cached

	const hostAddress = CHAINS_BY_ISMP_HOST[chain]
	if (!hostAddress) {
		throw new Error(`No ISMP host address configured for chain: ${chain}`)
	}

	const address = (await EthereumHostAbi__factory.connect(hostAddress, api).feeToken()).toLowerCase()
	const decimals = await ERC6160Ext20Abi__factory.connect(address, api).decimals()

	const feeToken = { address, decimals }
	feeTokenCache.set(chain, feeToken)
	return feeToken
}
