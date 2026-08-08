import { EthereumHostAbi__factory } from "@/configs/src/types/contracts"
import { CHAINS_BY_ISMP_HOST } from "@/constants"

// The host's fee token is static per chain, so one contract call per process suffices.
const feeTokenCache = new Map<string, string>()

/**
 * Returns the fee token address of a chain's ISMP host contract, cached per chain.
 * @param chain The chain's state machine identifier (e.g., "EVM-56")
 * @throws Error if no host address is configured for the chain or the contract call fails
 */
export async function getHostFeeToken(chain: string): Promise<string> {
	const cached = feeTokenCache.get(chain)
	if (cached) return cached

	const hostAddress = CHAINS_BY_ISMP_HOST[chain]
	if (!hostAddress) {
		throw new Error(`No ISMP host address configured for chain: ${chain}`)
	}

	const feeToken = (await EthereumHostAbi__factory.connect(hostAddress, api).feeToken()).toLowerCase()
	feeTokenCache.set(chain, feeToken)
	return feeToken
}
