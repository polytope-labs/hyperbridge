import { erc20Abi } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildCirclePaymasterData } from "./provider/circle"
import { buildSimplexPaymasterData } from "./provider/simplex"
import { packPaymasterAndData } from "./types"
import type { PaymasterOptions, PaymasterDataResult } from "./types"

export type { PaymasterOptions, PaymasterDataResult } from "./types"

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
 * 1. Circle Paymaster — when configured AND solver has ≥1 USDC balance
 * 2. Simplex Paymaster — when configured AND solver has ≥1 balance in USDC or USDT
 * 3. None — returns "0x" with a reason (caller falls back to EntryPoint deposit)
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

	if (circleAddr) {
		const usdcAddress = configService.getUsdcAsset(chain)
		const usdcDecimals = configService.getUsdcDecimals(chain)

		if (usdcAddress && usdcAddress !== "0x") {
			const { sufficient } = await getUsdcBalanceStatus(publicClient, solverAccount, usdcAddress, usdcDecimals)
			if (sufficient) {
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
		}
	}

	if (simplexAddr) {
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
	}

	return {
		paymasterAndData: "0x" as HexString,
		type: "none",
		reason: "insufficient stablecoin balance for all configured paymasters",
	}
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
