import type { NativeSetupNeed } from "@/services/native-setup"
import { RECOMMENDED_AMOUNT_USD } from "@/services/paymaster/types"

/**
 * The fee token a chain needs before fills are enabled: what one Permit2 signature lets the
 * paymaster take for an op's gas. It takes far less in practice and refunds what goes unused.
 */
export const MIN_FEE_TOKEN_BALANCE = Number(RECOMMENDED_AMOUNT_USD)

/** Whether a chain can fill: the gateway fee token, the native its paymaster setup needs, and the delegation. */
export interface ChainFunding {
	/** The gateway's fee token: USDC on most chains, USDT on BNB Chain. */
	feeToken: { symbol: string; balance: number }
	/** Native the paymaster's one-time setup still needs, when the paymaster's token has no permit. */
	nativeSetup: NativeSetupNeed | null
	/** The wallet holds {@link MIN_FEE_TOKEN_BALANCE} of the fee token and any native its setup needs. */
	funded: boolean
	/** The wallet is delegated to the SolverAccount here, so fills are enabled. */
	delegated: boolean
}

/** What `funding` still lacks, as "at least 5 USDT and about 0.00014 BNB"; undefined once it is funded. */
export function fundingShortfall(funding: ChainFunding, nativeSymbol: string): string | undefined {
	if (funding.funded) return undefined
	const needs = [
		funding.feeToken.balance >= MIN_FEE_TOKEN_BALANCE
			? undefined
			: `at least ${MIN_FEE_TOKEN_BALANCE} ${funding.feeToken.symbol}`,
		funding.nativeSetup
			? `about ${funding.nativeSetup.required.toLocaleString("en-US", { maximumSignificantDigits: 2 })} ${nativeSymbol}`
			: undefined,
	].filter(Boolean)
	return needs.join(" and ")
}
