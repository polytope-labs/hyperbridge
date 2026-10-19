import { encodeFunctionData, keccak256, toHex } from "viem"
import type { ERC7821Call, HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { ContractInteractionService } from "@/services/ContractInteractionService"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Logger } from "@/services/Logger"
import type { TokenSender } from "@/services/TokenSender"

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
	const chains = configService
		.getConfiguredChainIds()
		.filter((chainId) => deps.watchOnly?.[chainId] !== true)
		.map((chainId) => `EVM-${chainId}`)

	// Chains run side by side, but one chain's tokens go one after another: two native
	// transactions from the same wallet would race for a nonce, and so would two ops on the one
	// revoke nonce key. Another chain has its own EntryPoint and wallet nonce, so it never shares them.
	await Promise.all(
		chains.map(async (chain) => {
			try {
				await revokeOnChain(deps, chain)
			} catch (err) {
				logger.warn({ chain, err }, "Could not check the gateway allowance, leaving it as it is")
			}
		}),
	)
}

async function revokeOnChain(deps: StaleGatewayAllowanceDeps, chain: string): Promise<void> {
	const { clientManager, configService, contractService, tokenSender, solver, logger } = deps
	const client = clientManager.getPublicClient(chain)
	const gateway = configService.getIntentGatewayAddress(chain)

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

	for (const token of tokens.values()) {
		try {
			const allowance = (await client.readContract({
				address: token,
				abi: ERC20_ABI,
				functionName: "allowance",
				args: [solver, gateway],
			})) as bigint
			if (allowance === 0n) continue

			const revoke: ERC7821Call = {
				target: token,
				value: 0n,
				data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [gateway, 0n] }),
			}
			const { txHash, sponsored } = await tokenSender.submitCalls(chain, [revoke], {
				nonceKey: REVOKE_NONCE_KEY,
			})
			logger.info(
				{ chain, token, gateway, allowance: allowance.toString(), txHash, sponsored },
				"Revoked a stale IntentGateway allowance",
			)
		} catch (err) {
			logger.warn({ chain, token, gateway, err }, "Could not revoke a stale IntentGateway allowance")
		}
	}
}
