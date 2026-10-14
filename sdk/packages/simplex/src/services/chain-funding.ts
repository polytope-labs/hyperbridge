import type { NativeSetupNeed } from "@/services/native-setup"

/** Whether a chain can fill: the gateway fee token, the native its paymaster setup needs, and the delegation. */
export interface ChainFunding {
	/** The gateway's fee token: USDC on most chains, USDT on BNB Chain. */
	feeToken: { symbol: string; balance: number }
	/** Native the paymaster's one-time setup still needs, when the paymaster's token has no permit. */
	nativeSetup: NativeSetupNeed | null
	/** The wallet holds the fee token and any native its setup needs. */
	funded: boolean
	/** The wallet is delegated to the SolverAccount here, so fills are enabled. */
	delegated: boolean
}

/** What `funding` still lacks, as "USDT and about 0.00014 BNB"; undefined once it is funded. */
export function fundingShortfall(funding: ChainFunding, nativeSymbol: string): string | undefined {
	if (funding.funded) return undefined
	const needs = [
		funding.feeToken.balance > 0 ? undefined : funding.feeToken.symbol,
		funding.nativeSetup
			? `about ${funding.nativeSetup.required.toLocaleString("en-US", { maximumSignificantDigits: 2 })} ${nativeSymbol}`
			: undefined,
	].filter(Boolean)
	return needs.join(" and ")
}
