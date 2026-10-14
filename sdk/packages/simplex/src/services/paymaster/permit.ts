import {
	BaseError,
	ContractFunctionRevertedError,
	ContractFunctionZeroDataError,
	hashDomain,
	maxUint256,
	pad,
	toHex,
	type PublicClient,
	type TypedData,
	type TypedDataDomain,
	type TypedDataParameter,
} from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { EIP2612_ABI } from "@/config/abis/EIP2612"
import { normalizeSignature65 } from "./permit2"
import type { Signer } from "@/services/wallet/types"

/** The EIP-712 domain a token's `permit` verifies against, with its field list. */
export interface PermitDomain {
	domain: TypedDataDomain
	fields: TypedDataParameter[]
}

/** The domain shapes tokens use for EIP-2612, each built from the token's name and version. */
const DOMAIN_SHAPES: Array<{
	fields: TypedDataParameter[]
	domain: (name: string, version: string, chainId: number, token: HexString) => TypedDataDomain
}> = [
	{
		fields: [
			{ name: "name", type: "string" },
			{ name: "version", type: "string" },
			{ name: "chainId", type: "uint256" },
			{ name: "verifyingContract", type: "address" },
		],
		domain: (name, version, chainId, verifyingContract) => ({ name, version, chainId, verifyingContract }),
	},
	// USDT0 on Polygon: the chain id goes in a salt instead of a chainId field.
	{
		fields: [
			{ name: "name", type: "string" },
			{ name: "version", type: "string" },
			{ name: "verifyingContract", type: "address" },
			{ name: "salt", type: "bytes32" },
		],
		domain: (name, version, chainId, verifyingContract) => ({
			name,
			version,
			verifyingContract,
			salt: pad(toHex(chainId), { size: 32 }),
		}),
	},
]

/** Versions to try for a token with no `version()`: USDT0 on Polygon and Arbitrum signs as "1". */
const FALLBACK_VERSIONS = ["1", "2"]

/**
 * Reads a view the token may not have. A revert or empty return means it has none; any other
 * failure, a 429 included, is thrown, so a flaky RPC is never mistaken for a token without a permit.
 */
async function readIfPresent<T>(read: () => Promise<T>): Promise<T | undefined> {
	try {
		return await read()
	} catch (error) {
		const isRevert =
			error instanceof BaseError &&
			error.walk((e) => e instanceof ContractFunctionRevertedError || e instanceof ContractFunctionZeroDataError)
		if (isRevert) return undefined
		throw error
	}
}

/**
 * The EIP-712 domain a token's permit checks, found by matching its on-chain `DOMAIN_SEPARATOR`.
 *
 * Tokens disagree on the domain. Most use name, version, chainId and verifyingContract. USDT0 on
 * Polygon puts the chain id in a `salt` instead, and USDT0 has no `version()` on Polygon or
 * Arbitrum. So each known shape is hashed, and only one that equals the separator is used.
 * Returns null when the token has no `DOMAIN_SEPARATOR` or none of the shapes matches it: Simplex
 * cannot sign a permit that token would accept.
 */
export async function resolvePermitDomain(
	client: PublicClient,
	tokenAddress: HexString,
	chainId: number,
): Promise<PermitDomain | null> {
	const read = (functionName: "DOMAIN_SEPARATOR" | "name" | "version") =>
		readIfPresent(() => client.readContract({ address: tokenAddress, abi: EIP2612_ABI, functionName }))

	const separator = (await read("DOMAIN_SEPARATOR")) as HexString | undefined
	if (!separator) return null
	const name = (await read("name")) as string | undefined
	if (name === undefined) return null
	const version = (await read("version")) as string | undefined

	for (const candidate of version === undefined ? FALLBACK_VERSIONS : [version]) {
		for (const shape of DOMAIN_SHAPES) {
			const domain = shape.domain(name, candidate, chainId, tokenAddress)
			const hash = hashDomain<TypedData>({ domain, types: { EIP712Domain: shape.fields } })
			if (hash.toLowerCase() === separator.toLowerCase()) return { domain, fields: shape.fields }
		}
	}
	return null
}

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
	const permitDomain = await resolvePermitDomain(client, tokenAddress, chainId)
	if (!permitDomain) {
		throw new Error(`${tokenAddress} on chain ${chainId} has no EIP-2612 permit domain Simplex can sign for`)
	}
	const nonce = await client.readContract({
		address: tokenAddress,
		abi: EIP2612_ABI,
		functionName: "nonces",
		args: [owner],
	})

	const typedData = {
		types: {
			// Listed explicitly: custody backends hash the domain from this entry, not from `domain`.
			EIP712Domain: permitDomain.fields,
			Permit: [
				{ name: "owner", type: "address" },
				{ name: "spender", type: "address" },
				{ name: "value", type: "uint256" },
				{ name: "nonce", type: "uint256" },
				{ name: "deadline", type: "uint256" },
			],
		},
		primaryType: "Permit" as const,
		domain: permitDomain.domain,
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
