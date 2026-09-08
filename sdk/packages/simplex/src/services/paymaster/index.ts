import type { HexString } from "@hyperbridge/sdk"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildSimplexPaymasterData } from "./provider/simplex"
import {
	DEPOSIT_HEADROOM_PERCENT,
	packPaymasterAndData,
	POST_OP_GAS_LIMIT_SIMPLEX,
	VERIFICATION_GAS_LIMIT_PERMIT2,
} from "./types"
import type { FeeTokenBalance, PaymasterOptions, PaymasterDataResult } from "./types"

export type { PaymasterOptions, PaymasterDataResult, PaymasterPrefund } from "./types"

/**
 * Returns true if the chain has the Simplex paymaster configured. Used by filler.ts
 * to decide whether to skip EntryPoint deposits.
 */
export function hasPaymaster(chain: string, configService: FillerConfigService): boolean {
	return !!configService.getSimplexPaymasterAddress(chain)
}

/**
 * Paymaster data builder.
 *
 * The Simplex paymaster is the only sponsor: it is used when configured AND its
 * EntryPoint deposit covers the op's max prefund AND the solver holds ≥1 whole unit
 * of USDC or USDT. Otherwise this returns "0x" with a reason and the caller falls
 * back to the EntryPoint deposit.
 *
 * The deposit gate only runs when the caller passes `prefund` and the chain has an
 * EntryPoint configured; without either, selection is balance-only.
 */
export async function buildPaymasterAndData(options: PaymasterOptions): Promise<PaymasterDataResult> {
	const { chain, solverAccount, publicClient, walletClient, signer, configService } = options

	const simplexAddr = configService.getSimplexPaymasterAddress(chain)
	if (!simplexAddr) {
		return { paymasterAndData: "0x" as HexString, type: "none", reason: "no paymaster configured" }
	}

	// Checked before the builder: buildSimplexPaymasterData can send a bootstrap
	// approve tx, which must not happen for a paymaster that cannot sponsor.
	const shortfall = await depositShortfall(
		options,
		simplexAddr,
		VERIFICATION_GAS_LIMIT_PERMIT2 + POST_OP_GAS_LIMIT_SIMPLEX,
	)
	if (shortfall) {
		return { paymasterAndData: "0x" as HexString, type: "none", reason: shortfall }
	}

	// A builder failure (RPC error, bootstrap approve revert, missing native dust)
	// becomes a reason rather than a throw: the caller falls back to paying native.
	try {
		const pm = await buildSimplexPaymasterData(
			publicClient,
			walletClient,
			signer,
			solverAccount,
			simplexAddr,
			chain,
			configService,
		)
		if ("paymaster" in pm) {
			return {
				paymasterAndData: packPaymasterAndData(pm),
				type: "simplex",
				address: simplexAddr,
				token: pm.token,
			}
		}
		return {
			paymasterAndData: "0x" as HexString,
			type: "none",
			reason: `simplex: ${describeShortfall(pm.insufficient)}`,
		}
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		options.logger?.warn({ chain, error }, "Simplex paymaster builder failed; falling back to native gas")
		return { paymasterAndData: "0x" as HexString, type: "none", reason: `simplex: ${message}` }
	}
}

/**
 * Returns a skip reason when `paymaster`'s EntryPoint deposit cannot cover this
 * op's max prefund with {@link DEPOSIT_HEADROOM_PERCENT} headroom, undefined when
 * it can. `pmGas` is the paymaster's worst-case verification + postOp gas.
 *
 * Fails open (undefined) when the check cannot run — no prefund info, no
 * EntryPoint configured, or a failed read: a transient RPC error must not disable
 * sponsorship on a healthy chain, and the worst case is today's bundler rejection.
 */
async function depositShortfall(
	options: PaymasterOptions,
	paymaster: HexString,
	pmGas: bigint,
): Promise<string | undefined> {
	const { chain, publicClient, configService, prefund, logger } = options
	if (!prefund) return undefined
	const entryPoint = configService.getEntryPointAddress(chain)
	if (!entryPoint) return undefined

	const required = ((prefund.baseGas + pmGas) * prefund.maxFeePerGas * DEPOSIT_HEADROOM_PERCENT) / 100n

	let deposit: bigint
	try {
		deposit = (await publicClient.readContract({
			address: entryPoint,
			abi: ENTRYPOINT_ABI,
			functionName: "balanceOf",
			args: [paymaster],
		})) as bigint
	} catch (error) {
		logger?.warn(
			{ chain, paymaster, error },
			"Failed to read simplex paymaster EntryPoint deposit; assuming sufficient",
		)
		return undefined
	}

	if (deposit >= required) return undefined

	logger?.warn(
		{ chain, paymaster, deposit: deposit.toString(), required: required.toString() },
		"Skipping simplex paymaster: EntryPoint deposit below required prefund",
	)
	return `simplex: EntryPoint deposit ${deposit} < ${required} required`
}

// ── Wallet reserve ───────────────────────────────────────────────────

/**
 * Whole tokens of a paymaster-eligible stablecoin a fill must leave in the
 * wallet. The paymaster's pull is the EntryPoint's worst-case gas cost — cents
 * on an L2, most of it refunded in postOp — so this is mostly headroom for gas
 * spikes and for other UserOps in flight against the same balance.
 */
export const PAYMASTER_RESERVE_TOKENS = 2n

/**
 * Wallet balance of `tokenLower` that a fill on `chain` must not spend, because
 * the paymaster charges gas in this token and pulls it from the same wallet
 * during validatePaymasterUserOp — before the UserOp's callData runs. A fill
 * sized to the whole balance is therefore always short by that pull.
 *
 * Every eligible token carries the reserve, not just the one that ends up
 * charged: {@link buildPaymasterAndData} chooses between them at submit time
 * from live balances, and the fill sizing that consults this is one of the
 * inputs to that choice, so there is no winner to predict here.
 *
 * Returns 0 for a chain with no paymaster and for any token it cannot charge in.
 */
export function paymasterReserveForToken(
	chain: string,
	tokenLower: string,
	configService: FillerConfigService,
): bigint {
	if (!hasPaymaster(chain, configService)) return 0n

	const candidates: [HexString, () => number][] = [
		[configService.getUsdcAsset(chain), () => configService.getUsdcDecimals(chain)],
		[configService.getUsdtAsset(chain), () => configService.getUsdtDecimals(chain)],
	]

	for (const [address, decimals] of candidates) {
		if (!isConfiguredAsset(address)) continue
		if (address.toLowerCase() !== tokenLower) continue
		return PAYMASTER_RESERVE_TOKENS * 10n ** BigInt(decimals())
	}

	return 0n
}

/** Unconfigured assets come back from the config service as "0x" or the zero address. */
function isConfiguredAsset(address: HexString | undefined): address is HexString {
	return !!address && address !== "0x" && address.toLowerCase() !== "0x0000000000000000000000000000000000000000"
}

// ── Helpers ──────────────────────────────────────────────────────────

/**
 * Skip reason for a Simplex builder that found no fee token to charge: one clause per
 * token it read, in the form `solver USDC balance X < Y`.
 */
function describeShortfall(balances: FeeTokenBalance[]): string {
	if (balances.length === 0) return "no fee token configured"
	return `solver ${balances.map((b) => `${b.symbol} balance ${b.balance} < ${b.required}`).join(", ")}`
}
