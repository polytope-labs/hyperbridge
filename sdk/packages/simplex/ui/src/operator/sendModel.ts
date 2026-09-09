import { nativeTokenSymbol } from "@/cli/init/chains"
import type { SendTokenOption } from "../types"

/** Token choices shown by Send, including the loading fallback before config arrives. */
export function sendTokenOptionsForChain(
	options: SendTokenOption[] | undefined,
	chainId: number | null,
): SendTokenOption[] {
	const nativeSymbol = nativeTokenSymbol(chainId ?? 0)
	const available = options ?? [{ symbol: nativeSymbol, address: "native" }]
	return available.map((option) =>
		option.address === "native" && option.symbol !== nativeSymbol ? { ...option, symbol: nativeSymbol } : option,
	)
}

/** Human-readable picker label without changing the token symbol used by balances and icons. */
export function sendTokenOptionLabel(option: SendTokenOption): string {
	return option.address === "native" ? `${option.symbol} (native)` : option.symbol
}
