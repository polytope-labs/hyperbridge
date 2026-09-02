import { erc20Abi } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildCirclePaymasterData } from "./provider/circle"
import { buildSimplexPaymasterData } from "./provider/simplex"
import {
	DEPOSIT_HEADROOM_PERCENT,
	packPaymasterAndData,
	POST_OP_GAS_LIMIT_CIRCLE,
	POST_OP_GAS_LIMIT_SIMPLEX,
	VERIFICATION_GAS_LIMIT_CIRCLE,
	VERIFICATION_GAS_LIMIT_PERMIT,
} from "./types"
import type { PaymasterOptions, PaymasterDataResult } from "./types"

export type { PaymasterOptions, PaymasterDataResult, PaymasterPrefund } from "./types"

/**
 * Returns true if the chain has any paymaster (Circle or Simplex) configured.
 * Used by filler.ts to decide whether to skip EntryPoint deposits.
 */
export function hasPaymaster(chain: string, configService: FillerConfigService): boolean {
	return !!(configService.getCirclePaymasterAddress(chain) || configService.getSimplexPaymasterAddress(chain))
}

/**
 * Unified paymaster data builder.
 *
 * Selection:
 * 1. Simplex Paymaster — when configured AND its EntryPoint deposit covers the op's
 *    max prefund AND solver has ≥1 balance in USDC or USDT
 * 2. Circle Paymaster — when configured AND solver has ≥1 USDC balance AND its
 *    EntryPoint deposit covers the op's max prefund
 * 3. None — returns "0x" with a reason (caller falls back to EntryPoint deposit)
 *
 * The deposit gate only runs when the caller passes `prefund` and the chain has an
 * EntryPoint configured; without either, selection is balance-only as before.
 */
export async function buildPaymasterAndData(options: PaymasterOptions): Promise<PaymasterDataResult> {
	const {
		chain,
		solverAccount,
		publicClient,
		walletClient,
		signer,
		configService,
		paymasterVerificationGasLimit,
		skipPermit,
	} = options

	const circleAddr = configService.getCirclePaymasterAddress(chain)
	const simplexAddr = configService.getSimplexPaymasterAddress(chain)

	if (!circleAddr && !simplexAddr) {
		return { paymasterAndData: "0x" as HexString, type: "none", reason: "no paymaster configured" }
	}

	const skipReasons: string[] = []

	if (simplexAddr) {
		// Checked before the builder: buildSimplexPaymasterData can send a bootstrap
		// approve tx, which must not happen for a paymaster that cannot sponsor.
		const shortfall = await depositShortfall(
			options,
			simplexAddr,
			VERIFICATION_GAS_LIMIT_PERMIT + POST_OP_GAS_LIMIT_SIMPLEX,
			"simplex",
		)
		if (!shortfall) {
			// A builder failure (RPC error, bootstrap approve revert, missing native
			// dust) demotes Simplex to a skip reason instead of aborting selection —
			// Circle must still get its chance.
			try {
				const pm = await buildSimplexPaymasterData(
					publicClient,
					walletClient,
					signer,
					solverAccount,
					simplexAddr,
					chain,
					configService,
					{ skipPermit },
				)
				if (pm) {
					return {
						paymasterAndData: packPaymasterAndData(pm),
						type: "simplex",
						address: simplexAddr,
						token: pm.token,
					}
				}
				skipReasons.push("simplex: insufficient stablecoin balance")
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				options.logger?.warn({ chain, error }, "Simplex paymaster builder failed; trying next candidate")
				skipReasons.push(`simplex: ${message}`)
			}
		} else {
			skipReasons.push(shortfall)
		}
	}

	if (circleAddr) {
		const usdcAddress = configService.getUsdcAsset(chain)
		const usdcDecimals = configService.getUsdcDecimals(chain)

		if (usdcAddress && usdcAddress !== "0x") {
			const { balance, required, sufficient } = await getUsdcBalanceStatus(
				publicClient,
				solverAccount,
				usdcAddress,
				usdcDecimals,
			)
			if (sufficient) {
				// The override only ever lowers the signed limit, so the default is the
				// worst case the deposit must cover.
				const shortfall = await depositShortfall(
					options,
					circleAddr,
					VERIFICATION_GAS_LIMIT_CIRCLE + POST_OP_GAS_LIMIT_CIRCLE,
					"circle",
				)
				if (!shortfall) {
					const pm = await buildCirclePaymasterData(
						publicClient,
						signer,
						solverAccount,
						circleAddr,
						chain,
						configService,
						paymasterVerificationGasLimit,
					)
					return {
						paymasterAndData: packPaymasterAndData(pm),
						type: "circle",
						address: circleAddr,
						token: usdcAddress,
					}
				}
				skipReasons.push(shortfall)
			} else {
				skipReasons.push(`circle: solver USDC balance ${balance} < ${required}`)
			}
		}
	}

	return {
		paymasterAndData: "0x" as HexString,
		type: "none",
		reason:
			skipReasons.length > 0
				? skipReasons.join("; ")
				: "insufficient stablecoin balance for all configured paymasters",
	}
}

/**
 * Returns a skip reason when `paymaster`'s EntryPoint deposit cannot cover this
 * op's max prefund with {@link DEPOSIT_HEADROOM_PERCENT} headroom, undefined when
 * it can. `pmGas` is the candidate's worst-case verification + postOp gas.
 *
 * Fails open (undefined) when the check cannot run — no prefund info, no
 * EntryPoint configured, or a failed read: a transient RPC error must not disable
 * sponsorship on a healthy chain, and the worst case is today's bundler rejection.
 */
async function depositShortfall(
	options: PaymasterOptions,
	paymaster: HexString,
	pmGas: bigint,
	label: "circle" | "simplex",
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
		logger?.warn({ chain, paymaster, error }, `Failed to read ${label} paymaster EntryPoint deposit; assuming sufficient`)
		return undefined
	}

	if (deposit >= required) return undefined

	logger?.warn(
		{ chain, paymaster, deposit: deposit.toString(), required: required.toString() },
		`Skipping ${label} paymaster: EntryPoint deposit below required prefund`,
	)
	return `${label}: EntryPoint deposit ${deposit} < ${required} required`
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
 * Reads `account`'s token balance and reports it against the 1-token minimum the paymaster
 * needs to sponsor a UserOp. Returns the raw balance and required amount too, so callers can
 * log a precise deficit rather than a bare boolean.
 */
export async function getUsdcBalanceStatus(
	publicClient: PaymasterOptions["publicClient"],
	account: HexString,
	tokenAddress: HexString,
	tokenDecimals: number,
): Promise<{ balance: bigint; required: bigint; sufficient: boolean }> {
	const balance = (await publicClient.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "balanceOf",
		args: [account],
	})) as bigint

	const required = 10n ** BigInt(tokenDecimals)
	return { balance, required, sufficient: balance >= required }
}
