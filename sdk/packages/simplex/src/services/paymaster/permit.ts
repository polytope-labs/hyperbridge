import { maxUint256, getContract, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { EIP2612_ABI } from "@/config/abis/EIP2612"

/**
 * Signs an EIP-2612 permit for USDC, granting a spending allowance to the
 * Circle Paymaster via off-chain signature rather than an on-chain approve.
 *
 * Uses `deadline = maxUint256` because paymaster contracts cannot access
 * `block.timestamp` due to ERC-4337 opcode restrictions.
 */
export async function signEip2612Permit(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
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

	return signer.signTypedData(typedData, chainId)
}
