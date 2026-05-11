import { ERC20_ABI } from "@/config/abis/ERC20";
import type { HexString } from "@hyperbridge/sdk";
import { type Currency, Ether, Token } from "@uniswap/sdk-core";
import type { PublicClient } from "viem";
import { zeroAddress } from "viem";

/**
 * Resolve a chain identifier (e.g. "EVM-8453") to a numeric chain ID
 * used by the Uniswap SDK's Token / Pool constructors.
 */
export function chainIdFromIdentifier(chain: string): number {
	const parts = chain.split("-");
	const num = Number(parts[parts.length - 1]);
	if (Number.isNaN(num))
		throw new Error(`Cannot parse chainId from "${chain}"`);
	return num;
}

export function isNativePoolCurrency(address: HexString): boolean {
	return address.toLowerCase() === zeroAddress.toLowerCase();
}

/**
 * Native pool currency uses SDK native decimals; ERC-20s read `decimals()` on-chain.
 */
export async function fetchPoolCurrencyDecimals(
	client: PublicClient,
	chainId: number,
	address: HexString,
): Promise<number> {
	if (isNativePoolCurrency(address)) return Ether.onChain(chainId).decimals;
	const decimalsRaw = await client.readContract({
		address,
		abi: ERC20_ABI,
		functionName: "decimals",
	});
	return Number(decimalsRaw);
}

export function currencyFromHydratedDecimals(
	chainId: number,
	address: HexString,
	decimals: number,
): Currency {
	if (isNativePoolCurrency(address)) return Ether.onChain(chainId);
	return new Token(chainId, address, decimals);
}
