import { encodeFunctionData, keccak256, toHex } from "viem"
import type { ERC7821Call, HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { ContractInteractionService } from "@/services/ContractInteractionService"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Logger } from "@/services/Logger"
import type { TokenSender } from "@/services/TokenSender"

/** IntentGateway addresses a chain no longer points at, by chain ID. */
export type RetiredGateways = Readonly<Record<number, readonly HexString[]>>

/**
 * Mainnet IntentGateway addresses the SDK config has since replaced. The over-approval this sweep
 * undoes dates from 2026-03-11 (9149fc527), so every gateway the filler pointed at from then on
 * may still hold an allowance. Each address was the filler's gateway on the chains below, and the
 * dates are those of the SDK config commits that set it:
 *
 * - `0x2d61…` from 2b0e7fb12 (2026-03-05; Optimism and Gnosis from 2caeb37d6, 2026-03-12) to
 *   898e9e95a (2026-05-13, #825).
 * - `0x09a7…` from 898e9e95a to bf74af936 (2026-05-29, #933).
 * - `0x16F9…` from bf74af936 to f7e085486 (2026-06-25, #996), which moved mainnet to `0xAe04…`.
 *   Polkadot Hub took it from 76514f7e4 (2026-05-27) until the same move.
 */
const RETIRED_MAINNET_GATEWAYS: readonly HexString[] = [
	"0x2d61624A17f361020679FaA16fbB566C344AaF4B",
	"0x09a74AA91eb532C21862d4C6222753e4fC7e22dB",
	"0x16F9E57f735bBfF9f6c4E5276330f9c437d0e9E0",
]

/**
 * Testnet gateways the SDK config has since replaced: BSC Chapel and Polygon Amoy used
 * `0xFbF5…` from 9149fc527 (2026-03-11), `0xE13f…` from 713a589ba (2026-06-05, #950), and
 * `0x6CF4…` from 57982b94d (2026-06-30, #1009).
 */
const RETIRED_TESTNET_GATEWAYS: readonly HexString[] = [
	"0xFbF50B2b32768127603cC9eF4b871574b881b8eD",
	"0xE13fB34CAe12505ae51BaC8C405AF8EB27AC8058",
]

/**
 * Retired gateways per chain, from the history of `sdk/src/configs/chain.ts`. A chain that has
 * kept its gateway, or never had a retired one, is absent. A new gateway move adds the address it
 * leaves behind here, since an allowance to it outlives the config entry.
 */
export const RETIRED_GATEWAY_ADDRESSES: RetiredGateways = {
	1: RETIRED_MAINNET_GATEWAYS,
	10: RETIRED_MAINNET_GATEWAYS,
	56: RETIRED_MAINNET_GATEWAYS,
	100: RETIRED_MAINNET_GATEWAYS,
	137: RETIRED_MAINNET_GATEWAYS,
	8453: RETIRED_MAINNET_GATEWAYS,
	42161: RETIRED_MAINNET_GATEWAYS,
	420420419: ["0x16F9E57f735bBfF9f6c4E5276330f9c437d0e9E0"],
	97: RETIRED_TESTNET_GATEWAYS,
	80002: RETIRED_TESTNET_GATEWAYS,
}

export interface StaleGatewayAllowanceDeps {
	clientManager: Pick<ChainClientManager, "getPublicClient">
	configService: Pick<
		FillerConfigService,
		"getConfiguredChainIds" | "getIntentGatewayAddress" | "getUsdcAsset" | "getUsdtAsset"
	>
	contractService: Pick<ContractInteractionService, "getFeeTokenWithDecimals">
	tokenSender: Pick<TokenSender, "submitCalls">
	solver: HexString
	/** Chains the filler only observes: it never sends from them, so it does not here. */
	watchOnly?: Record<number, boolean>
	/** Defaults to {@link RETIRED_GATEWAY_ADDRESSES}. */
	retiredGateways?: RetiredGateways
	logger: Pick<Logger, "info" | "warn">
}

/**
 * The EntryPoint nonce key the revoke ops use. The revoke runs at boot, when the other key-0
 * senders do too (`bridgeTopUp.restore()`, then the first vault sweep about 30s later); two ops on
 * one key read the same `getNonce`, so the bundler would reject or replace one and the revoke would
 * fall back to a native send or time out. A lane of its own never competes with them.
 *
 * The EntryPoint's key is a uint192, so the 256-bit hash is shifted down to fit. It cannot meet a
 * bid's key: those are `bidNonceKey(commitment, session, callData)`, a keccak over 84 bytes of
 * order data, and `SolverAccount` only checks the key of a bid op (one with a session signature),
 * which this plain-signature op is not. A collision would need a second preimage of 192 bits.
 */
const REVOKE_NONCE_KEY = BigInt(keccak256(toHex("simplex.gateway-allowance-revoke"))) >> 64n

/** Missing assets come back from the config service as the literal "0x". */
function isConfigured(address: HexString): boolean {
	return !!address && address !== "0x" && address !== "0x0000000000000000000000000000000000000000"
}

/**
 * Revokes the IntentGateway allowance an older release left standing on the solver wallet.
 *
 * Fills used to approve the destination fee token for a figure in the source chain's decimals
 * (about 1.09e16 units of a 6-decimal USDC) and never reset it. A fill resets the allowance of
 * the tokens it touches, so the leftover would otherwise stand on a chain until the next fill in
 * that token there, and the gateway is an upgradeable proxy. This clears it at boot instead.
 *
 * The same code ran against the gateways the chain used before its current one, and moving to a
 * new gateway does not clear what was granted to the old one. So the sweep covers the current
 * gateway and every address in `retiredGateways` for the chain, other than one equal to the
 * current gateway. A chain with no retired address costs no extra reads.
 *
 * Reads `allowance(solver, gateway)` for the gateway's fee token and the configured USDC and USDT
 * on every configured chain that is not watch-only, and sends `approve(gateway, 0)` for each
 * that is not zero, one call per submission: `submitCalls` tries a sponsored op whenever a paymaster
 * and bundler are configured and sends natively only when the bundler refuses it, and a single call
 * goes straight to the token, so that fallback works from a wallet that is not delegated yet. The
 * sponsored op uses {@link REVOKE_NONCE_KEY}. Idempotent, since a revoked
 * allowance reads zero on the next boot. Best-effort: a failure is logged and the rest carry on,
 * so a flaky endpoint never blocks the filler from starting. Never rejects.
 */
export async function revokeStaleGatewayAllowances(deps: StaleGatewayAllowanceDeps): Promise<void> {
	const { configService, logger } = deps
	const chainIds = configService.getConfiguredChainIds().filter((chainId) => deps.watchOnly?.[chainId] !== true)
	const retiredGateways = deps.retiredGateways ?? RETIRED_GATEWAY_ADDRESSES

	// Chains run side by side, but one chain's revokes go one after another, over its tokens and
	// gateways alike: two native transactions from the same wallet would race for a nonce, and so
	// would two ops on the one revoke nonce key. Going in turn only keeps them apart while each op
	// confirms. If one is left unconfirmed, the next op on that key collides with it and fails, and
	// the next boot retries it. Another chain has its own EntryPoint and wallet nonce, so it never
	// shares them.
	await Promise.all(
		chainIds.map(async (chainId) => {
			const chain = `EVM-${chainId}`
			try {
				await revokeOnChain(deps, chain, retiredGateways[chainId] ?? [])
			} catch (err) {
				logger.warn({ chain, err }, "Could not check the gateway allowance, leaving it as it is")
			}
		}),
	)
}

async function revokeOnChain(
	deps: StaleGatewayAllowanceDeps,
	chain: string,
	retired: readonly HexString[],
): Promise<void> {
	const { clientManager, configService, contractService, tokenSender, solver, logger } = deps
	const client = clientManager.getPublicClient(chain)
	const gateway = configService.getIntentGatewayAddress(chain)
	// Distinct from the current gateway and from each other; the current one is swept first.
	const gateways = new Map<string, { address: HexString; retired: boolean }>([
		[gateway.toLowerCase(), { address: gateway, retired: false }],
	])
	for (const address of retired) {
		if (!gateways.has(address.toLowerCase())) gateways.set(address.toLowerCase(), { address, retired: true })
	}

	const tokens = new Map<string, HexString>()
	const add = (token: HexString) => {
		if (isConfigured(token)) tokens.set(token.toLowerCase(), token)
	}
	try {
		add((await contractService.getFeeTokenWithDecimals(chain)).address)
	} catch (err) {
		// USDC and USDT are still worth checking without it.
		logger.warn({ chain, err }, "Could not read the gateway's fee token, checking USDC and USDT only")
	}
	add(configService.getUsdcAsset(chain))
	add(configService.getUsdtAsset(chain))

	for (const { address: spender, retired: isRetired } of gateways.values()) {
		for (const token of tokens.values()) {
			try {
				const allowance = (await client.readContract({
					address: token,
					abi: ERC20_ABI,
					functionName: "allowance",
					args: [solver, spender],
				})) as bigint
				if (allowance === 0n) continue

				const revoke: ERC7821Call = {
					target: token,
					value: 0n,
					data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [spender, 0n] }),
				}
				const { txHash, sponsored } = await tokenSender.submitCalls(chain, [revoke], {
					nonceKey: REVOKE_NONCE_KEY,
				})
				logger.info(
					{ chain, token, gateway: spender, retired: isRetired, allowance: allowance.toString(), txHash, sponsored },
					"Revoked a stale IntentGateway allowance",
				)
			} catch (err) {
				logger.warn(
					{ chain, token, gateway: spender, retired: isRetired, err },
					"Could not revoke a stale IntentGateway allowance",
				)
			}
		}
	}
}
