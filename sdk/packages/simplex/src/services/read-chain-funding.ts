import { formatUnits, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { MIN_FEE_TOKEN_BALANCE, type ChainFunding } from "@/services/chain-funding"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { isDelegatedTo, nativeSetupNeed } from "@/services/native-setup"

interface FeeToken {
	address: HexString
	symbol: string
	decimals: number
}

// A host's fee token is fixed, so it is read once per chain.
const feeTokens = new Map<string, Promise<FeeToken>>()

/**
 * Reads what `wallet` holds toward filling on `chain`. Fills pay protocol fees in the
 * gateway's fee token and gas through the paymaster, so a chain is funded once the
 * wallet holds that token and any native the paymaster's one-time setup needs.
 */
export async function readChainFunding(
	client: PublicClient,
	configService: FillerConfigService,
	wallet: HexString,
	chain: string,
	nativeBalance?: bigint,
): Promise<ChainFunding> {
	const feeToken = await gatewayFeeToken(client, configService, chain)
	const [balance, native, code] = await Promise.all([
		client.readContract({ address: feeToken.address, abi: ERC20_ABI, functionName: "balanceOf", args: [wallet] }),
		nativeBalance ?? client.getBalance({ address: wallet }),
		client.getCode({ address: wallet }),
	])
	const held = Number(formatUnits(balance, feeToken.decimals))
	const holdsFeeToken = held >= MIN_FEE_TOKEN_BALANCE
	// Before the fee token arrives, its setup is judged on the fee token itself, so a chain whose
	// token has no permit (BNB Chain) asks for the token and the native together.
	const nativeSetup = await nativeSetupNeed(
		client,
		configService,
		wallet,
		chain,
		native,
		holdsFeeToken ? undefined : feeToken.address,
	)
	return {
		feeToken: { symbol: feeToken.symbol, balance: held },
		nativeSetup,
		funded: holdsFeeToken && !nativeSetup,
		delegated: isDelegatedTo(code, configService.getSolverAccountContractAddress(chain)),
	}
}

function gatewayFeeToken(client: PublicClient, configService: FillerConfigService, chain: string): Promise<FeeToken> {
	let pending = feeTokens.get(chain)
	if (!pending) {
		pending = readFeeToken(client, configService, chain)
		// A failed read is not cached, so the next call retries it.
		pending.catch(() => feeTokens.delete(chain))
		feeTokens.set(chain, pending)
	}
	return pending
}

async function readFeeToken(
	client: PublicClient,
	configService: FillerConfigService,
	chain: string,
): Promise<FeeToken> {
	const address = await client.readContract({
		address: configService.getHostAddress(chain),
		abi: EVM_HOST,
		functionName: "feeToken",
	})
	const [symbol, decimals] = await Promise.all([
		client.readContract({ address, abi: ERC20_ABI, functionName: "symbol" }),
		client.readContract({ address, abi: ERC20_ABI, functionName: "decimals" }),
	])
	return { address, symbol, decimals: Number(decimals) }
}
