import { maxUint256, getContract, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { EIP2612_ABI } from "@/config/abis/EIP2612"
import { normalizeSignature65 } from "./permit2"
import type { Signer } from "@/services/wallet/types"

/**
 * Signs an EIP-2612 permit granting `spender` an allowance via off-chain signature
 * rather than an on-chain approve.
 *
 * Simplex signs one of these only to bootstrap a chain: a first-time delegation on an
 * account with no Permit2 allowance yet. Every other sponsored op authorizes through
 * Permit2, whose unordered-bitmap nonces let concurrent ops proceed without sharing
 * the single sequential counter this signs against — see {@link signPermit2Transfer}.
 *
 * Uses `deadline = maxUint256` because paymaster contracts cannot access
 * `block.timestamp` due to ERC-4337 opcode restrictions.
 */
export async function signEip2612Permit(
	client: PublicClient,
	signer: Pick<Signer, "signTypedData">,
	owner: HexString,
	spender: HexString,
	tokenAddress: HexString,
	value: bigint,
	chainId: number,
): Promise<HexString> {
	const token = getContract({
		client,
		address: tokenAddress,
		abi: EIP2612_ABI,
	})

	const [name, version, nonce] = await Promise.all([
		token.read.name(),
		token.read.version(),
		token.read.nonces([owner]),
	])

	const typedData = {
		types: {
			EIP712Domain: [
				{ name: "name", type: "string" },
				{ name: "version", type: "string" },
				{ name: "chainId", type: "uint256" },
				{ name: "verifyingContract", type: "address" },
			],
			Permit: [
				{ name: "owner", type: "address" },
				{ name: "spender", type: "address" },
				{ name: "value", type: "uint256" },
				{ name: "nonce", type: "uint256" },
				{ name: "deadline", type: "uint256" },
			],
		},
		primaryType: "Permit" as const,
		domain: {
			name,
			version,
			chainId,
			verifyingContract: tokenAddress,
		},
		message: {
			owner,
			spender,
			value,
			nonce,
			deadline: maxUint256,
		},
	}

	// Same normalization the Permit2 signer applies: a backend may return a 64-byte EIP-2098
	// compact signature or v in {0,1}, and `buildPermitMode` splits v straight out of the hex
	// for a contract that expects {27,28}. A correct 65-byte signature passes through unchanged.
	return normalizeSignature65(await signer.signTypedData(typedData))
}
