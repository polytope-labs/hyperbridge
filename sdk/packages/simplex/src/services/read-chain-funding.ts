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

/**
 * {@link readChainFunding} through RPCs nothing has checked yet: the setup wizard's, typed by the
 * operator before any filler exists. The fee token here and the paymaster's Permit2 probe are
 * cached for the whole process, which the filler booted by the wizard shares, so a URL serving
 * another chain would pin that chain's answers for this one. Every URL is probed for its chain id
 * first and the read goes only through those that answered `chainId`:
 *
 * - a URL that answers another chain fails the read, since the operator typed it for this one;
 * - a URL that cannot be reached is left out, as the fallback transport would skip it anyway;
 * - with no URL answering at all, the read fails without touching the chain.
 */
export async function readChainFundingFromUnverifiedRpcs(
	rpcUrls: readonly string[],
	chainId: number,
	configService: FillerConfigService,
	wallet: HexString,
	io: {
		/** A bounded `eth_chainId` probe of one URL. */
		probeChainId: (url: string) => Promise<number>
		/** A client reading through `urls` with fallback between them. */
		connect: (urls: string[]) => PublicClient
	},
): Promise<ChainFunding> {
	const probes = await Promise.allSettled(rpcUrls.map((url) => io.probeChainId(url)))
	const verified: string[] = []
	probes.forEach((probe, index) => {
		if (probe.status === "rejected") return
		if (probe.value !== chainId) {
			throw new Error(`RPC endpoint ${index + 1} serves chain ${probe.value}, not chain ${chainId}`)
		}
		verified.push(rpcUrls[index])
	})
	if (verified.length === 0) {
		throw new Error(`None of the RPC endpoints for chain ${chainId} answered; check the URLs and try again`)
	}
	return readChainFunding(io.connect(verified), configService, wallet, `EVM-${chainId}`)
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
