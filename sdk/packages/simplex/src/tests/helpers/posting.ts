import type { HexString } from "@hyperbridge/sdk"
import { EvmChain, IntentGateway } from "@hyperbridge/sdk"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"
import type { Signer } from "@/services/wallet"

/**
 * The real posting path, over the few collaborators that would otherwise want a
 * node.
 *
 * Everything the orderbook reads out of an op is built by the code that builds
 * it in production. What is stood in for is the endpoints, which nothing on this
 * path reads, the fee token the gateway warms its cache with, and the token
 * decimals, which are the orderbook's own config here rather than something to
 * discover.
 */

/** Anvil's first account, which is the key the orderbook's vectors were signed with. */
export const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString

/** Base's own tokens, which both the vectors and the orderbook's dev config use. */
export const BASE_USDC = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" as HexString
export const BASE_CNGN = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" as HexString
export const BASE_CHAIN = "EVM-8453"
export const BASE_HOST = "0x6FFe92e4d7a9D589549644544780e6725E84b248" as HexString

const gateways = new Map<number, Promise<IntentGateway>>()

/** Built once per chain: the solver account lookup inside `create` waits on a node that is not there. */
function intentGateway(chainId: number): Promise<IntentGateway> {
	const existing = gateways.get(chainId)
	if (existing) return existing

	const chain = EvmChain.fromParams({ chainId, host: BASE_HOST, rpcUrl: "http://127.0.0.1:1" })
	// biome-ignore lint/suspicious/noExplicitAny: the fee token read is the one thing here that wants a node
	;(chain as any).getFeeTokenWithDecimals = async () => ({ address: BASE_USDC, decimals: 6 })
	const pending = IntentGateway.create(chain, chain)
	gateways.set(chainId, pending)
	return pending
}

export interface PostingRig {
	service: ContractInteractionService
	signer: Signer
}

export async function postingRig(params: {
	gateway: HexString
	chainId: number
	/** What both tokens use on Base, and what the orderbook is configured with. */
	decimals?: number
}): Promise<PostingRig> {
	const signer = privateKeySigner(SOLVER_KEY)
	const gateway = await intentGateway(params.chainId)
	const configService = {
		loggers: undefined,
		getConfiguredChainIds: () => [params.chainId],
		getIntentGatewayAddress: () => params.gateway,
		getEntryPointAddress: () => "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString,
		getRpcUrls: () => ["http://127.0.0.1:1"],
	}
	// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for the collaborators this path touches
	const service = new ContractInteractionService({} as any, configService as any, signer)
	// biome-ignore lint/suspicious/noExplicitAny: the gateway is built above rather than from a node
	;(service as any).getIntentGateway = async () => gateway
	// biome-ignore lint/suspicious/noExplicitAny: decimals are config here, not something to discover
	;(service as any).getTokenDecimals = async () => params.decimals ?? 6
	// Creation checks the wallet can pay out what the order promises. There is no
	// chain behind this rig, and what it exercises is the orderbook rather than the
	// balance rule, which has its own tests.
	// biome-ignore lint/suspicious/noExplicitAny: no node behind this rig
	;(service as any).getTokenBalance = async () => 10n ** 30n
	return { service, signer }
}

/** Resolves the two symbols the orderbook's dev book trades, and nothing else. */
export function baseAssetRegistry() {
	return {
		getAddress: (symbol: string, chain: string) =>
			chain === BASE_CHAIN ? ({ USDC: BASE_USDC, cNGN: BASE_CNGN })[symbol] ?? null : null,
	}
}
