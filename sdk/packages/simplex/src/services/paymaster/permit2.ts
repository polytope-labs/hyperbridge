import {
	bytesToBigInt,
	compactSignatureToSignature,
	parseCompactSignature,
	serializeSignature,
	size,
	type Hex,
} from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { Signer } from "@/services/wallet/types"

export interface Permit2TransferParams {
	permit2: HexString
	chainId: number
	token: HexString
	amount: bigint
	spender: HexString
	nonce: bigint
	deadline: bigint
}

/**
 * EIP-712 payload for a Permit2 `PermitTransferFrom`. Shaped like
 * `CryptoUtils.packedUserOpTypedData` (explicit `EIP712Domain`, numeric chainId)
 * so the MPC Vault and Turnkey signers hash it identically to viem.
 */
export function permit2TransferTypedData(p: Permit2TransferParams) {
	return {
		types: {
			EIP712Domain: [
				{ name: "name", type: "string" },
				{ name: "chainId", type: "uint256" },
				{ name: "verifyingContract", type: "address" },
			],
			PermitTransferFrom: [
				{ name: "permitted", type: "TokenPermissions" },
				{ name: "spender", type: "address" },
				{ name: "nonce", type: "uint256" },
				{ name: "deadline", type: "uint256" },
			],
			TokenPermissions: [
				{ name: "token", type: "address" },
				{ name: "amount", type: "uint256" },
			],
		},
		primaryType: "PermitTransferFrom" as const,
		domain: {
			name: "Permit2",
			chainId: p.chainId,
			verifyingContract: p.permit2,
		},
		message: {
			permitted: { token: p.token, amount: p.amount },
			spender: p.spender,
			nonce: p.nonce,
			deadline: p.deadline,
		},
	}
}

/**
 * Permit2 nonces are an unordered bitmap; a random 256-bit nonce lets concurrent
 * ops on one chain each carry their own permit without coordination.
 */
export function randomPermit2Nonce(): bigint {
	return bytesToBigInt(crypto.getRandomValues(new Uint8Array(32)))
}

/**
 * Signs a Permit2 transfer permit and returns a 65-byte (r, s, v) signature — the
 * only shape the ERC-1271 path through the delegated SolverAccount accepts.
 */
export async function signPermit2Transfer(
	signer: Pick<Signer, "signTypedData">,
	p: Permit2TransferParams,
): Promise<HexString> {
	const signature = await signer.signTypedData(permit2TransferTypedData(p))
	return normalizeSignature65(signature)
}

export function normalizeSignature65(signature: HexString): HexString {
	const length = size(signature as Hex)
	if (length === 64) {
		return serializeSignature(compactSignatureToSignature(parseCompactSignature(signature as Hex))) as HexString
	}
	if (length !== 65) {
		throw new Error(`Permit2 signature must be 64 or 65 bytes, got ${length}`)
	}
	const v = Number.parseInt(signature.slice(130, 132), 16)
	if (v === 0 || v === 1) {
		return `${signature.slice(0, 130)}${(v + 27).toString(16).padStart(2, "0")}` as HexString
	}
	return signature
}
