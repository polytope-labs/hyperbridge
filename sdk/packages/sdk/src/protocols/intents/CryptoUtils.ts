import {
	keccak256,
	toHex,
	encodeAbiParameters,
	decodeAbiParameters,
	decodeFunctionData,
	encodeFunctionData,
	concat,
	pad,
	encodePacked,
	parseAbiParameters,
	type Hex,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import type { HexString, PackedUserOperation } from "@/types"
import type { ERC7821Call } from "@/types"
import ERC7821ABI from "@/abis/erc7281"
import type { IntentGatewayContext } from "./types"
import { ERC7821_BATCH_MODE } from "./types"
import type { BundlerMethod } from "./types"

/**
 * EIP-712 type hash for the `SelectSolver` struct.
 *
 * Computed as `keccak256("SelectSolver(bytes32 commitment,address solver)")`.
 * Used when the session key signs a solver-selection message so that the
 * IntentGatewayV2 contract can verify the choice on-chain.
 */
export const SELECT_SOLVER_TYPEHASH = keccak256(toHex("SelectSolver(bytes32 commitment,address solver)"))

/**
 * EIP-712 type hash for the `PackedUserOperation` struct.
 *
 * Matches the ERC-4337 v0.8 `PackedUserOperation` type definition used by
 * EntryPoint v0.8. Used when computing the UserOperation hash that solvers
 * must sign before submitting bids.
 */
export const PACKED_USEROP_TYPEHASH = keccak256(
	toHex(
		"PackedUserOperation(address sender,uint256 nonce,bytes initCode,bytes callData,bytes32 accountGasLimits,uint256 preVerificationGas,bytes32 gasFees,bytes paymasterAndData)",
	),
)

/**
 * EIP-712 type hash for the `EIP712Domain` struct.
 *
 * Computed as `keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")`.
 * Used to construct domain separators for all EIP-712 messages in the
 * IntentGatewayV2 protocol.
 */
export const DOMAIN_TYPEHASH = keccak256(
	toHex("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
)

/**
 * Crypto and encoding utilities for IntentGatewayV2.
 *
 * Provides helpers for EIP-712 domain separation, UserOperation hashing,
 * gas-limit packing/unpacking, bundler JSON-RPC calls, and ERC-7821
 * batch-execute encoding and decoding. All methods are stateless with respect
 * to the protocol but require the shared {@link IntentGatewayContext} for the
 * bundler URL.
 */
export class CryptoUtils {
	/**
	 * @param ctx - Shared IntentsV2 context; used to access the bundler URL for
	 *   JSON-RPC calls.
	 */
	constructor(private readonly ctx: IntentGatewayContext) {}

	/**
	 * Computes an EIP-712 domain separator for a given contract.
	 *
	 * @param contractName - Human-readable name of the contract (e.g. `"IntentGateway"`).
	 * @param version - Version string (e.g. `"2"`).
	 * @param chainId - Chain ID of the network the contract is deployed on.
	 * @param contractAddress - Address of the verifying contract.
	 * @returns The 32-byte domain separator as a hex string.
	 */
	getDomainSeparator(contractName: string, version: string, chainId: bigint, contractAddress: HexString): HexString {
		return keccak256(
			encodeAbiParameters(parseAbiParameters("bytes32, bytes32, bytes32, uint256, address"), [
				DOMAIN_TYPEHASH,
				keccak256(toHex(contractName)),
				keccak256(toHex(version)),
				chainId,
				contractAddress,
			]),
		)
	}

	/**
	 * Signs a `SelectSolver` EIP-712 message with a session key.
	 *
	 * The session key authorises the selection of a specific solver for the
	 * given order commitment. The resulting signature is appended to the
	 * solver's UserOperation signature before bundle submission.
	 *
	 * @param commitment - The order commitment (bytes32) being fulfilled.
	 * @param solverAddress - Address of the solver account selected to fill the order.
	 * @param domainSeparator - EIP-712 domain separator for the IntentGatewayV2 contract.
	 * @param privateKey - Hex-encoded private key of the session key that signs the message.
	 * @returns The ECDSA signature as a hex string, or `null` if signing fails.
	 */
	async signSolverSelection(
		commitment: HexString,
		solverAddress: HexString,
		domainSeparator: HexString,
		privateKey: HexString,
	): Promise<HexString | null> {
		const account = privateKeyToAccount(privateKey as Hex)

		const structHash = keccak256(
			encodeAbiParameters(
				[{ type: "bytes32" }, { type: "bytes32" }, { type: "address" }],
				[SELECT_SOLVER_TYPEHASH, commitment, solverAddress],
			),
		)

		const digest = keccak256(concat(["0x1901" as Hex, domainSeparator as Hex, structHash]))
		const signature = await account.sign({ hash: digest })

		return signature as HexString
	}

	/**
	 * Computes the EIP-712 hash of a `PackedUserOperation` as defined by
	 * ERC-4337 EntryPoint v0.8.
	 *
	 * @param userOp - The packed UserOperation to hash.
	 * @param entryPoint - Address of the EntryPoint v0.8 contract.
	 * @param chainId - Chain ID of the network on which the operation will execute.
	 * @returns The UserOperation hash as a hex string.
	 */
	computeUserOpHash(userOp: PackedUserOperation, entryPoint: Hex, chainId: bigint): Hex {
		const structHash = this.getPackedUserStructHash(userOp)
		const domainSeparator = this.getDomainSeparator("ERC4337", "1", chainId, entryPoint as HexString)

		return keccak256(
			encodePacked(["bytes1", "bytes1", "bytes32", "bytes32"], ["0x19", "0x01", domainSeparator, structHash]),
		)
	}

	/**
	 * Computes the EIP-712 struct hash of a `PackedUserOperation`.
	 *
	 * Hashes dynamic fields (`initCode`, `callData`, `paymasterAndData`) before
	 * ABI-encoding so the final hash is a fixed-length 32-byte value.
	 *
	 * @param userOp - The packed UserOperation to hash.
	 * @returns The struct hash as a 32-byte hex string.
	 */
	getPackedUserStructHash(userOp: PackedUserOperation): HexString {
		return keccak256(
			encodeAbiParameters(
				parseAbiParameters("bytes32, address, uint256, bytes32, bytes32, bytes32, uint256, bytes32, bytes32"),
				[
					PACKED_USEROP_TYPEHASH,
					userOp.sender,
					userOp.nonce,
					keccak256(userOp.initCode),
					keccak256(userOp.callData),
					userOp.accountGasLimits as Hex,
					userOp.preVerificationGas,
					userOp.gasFees as Hex,
					keccak256(userOp.paymasterAndData),
				],
			),
		) as HexString
	}

	/**
	 * Packs `verificationGasLimit` and `callGasLimit` into the ERC-4337
	 * `accountGasLimits` bytes32 field.
	 *
	 * The high 16 bytes hold `verificationGasLimit` and the low 16 bytes hold
	 * `callGasLimit`, matching the EntryPoint v0.8 packed representation.
	 *
	 * @param verificationGasLimit - Gas limit for the account verification step.
	 * @param callGasLimit - Gas limit for the main execution call.
	 * @returns A 32-byte hex string with both limits packed.
	 */
	packGasLimits(verificationGasLimit: bigint, callGasLimit: bigint): HexString {
		const verificationGasHex = pad(toHex(verificationGasLimit), { size: 16 })
		const callGasHex = pad(toHex(callGasLimit), { size: 16 })
		return concat([verificationGasHex, callGasHex]) as HexString
	}

	/**
	 * Packs `maxPriorityFeePerGas` and `maxFeePerGas` into the ERC-4337
	 * `gasFees` bytes32 field.
	 *
	 * The high 16 bytes hold `maxPriorityFeePerGas` and the low 16 bytes hold
	 * `maxFeePerGas`, matching the EntryPoint v0.8 packed representation.
	 *
	 * @param maxPriorityFeePerGas - Maximum tip per gas (EIP-1559).
	 * @param maxFeePerGas - Maximum total fee per gas (EIP-1559).
	 * @returns A 32-byte hex string with both fee values packed.
	 */
	packGasFees(maxPriorityFeePerGas: bigint, maxFeePerGas: bigint): HexString {
		const priorityFeeHex = pad(toHex(maxPriorityFeePerGas), { size: 16 })
		const maxFeeHex = pad(toHex(maxFeePerGas), { size: 16 })
		return concat([priorityFeeHex, maxFeeHex]) as HexString
	}

	/**
	 * Unpacks the `accountGasLimits` bytes32 field back into its constituent
	 * gas limits.
	 *
	 * @param accountGasLimits - The packed 32-byte gas limits field from a `PackedUserOperation`.
	 * @returns Object with `verificationGasLimit` and `callGasLimit` as bigints.
	 */
	unpackGasLimits(accountGasLimits: HexString): { verificationGasLimit: bigint; callGasLimit: bigint } {
		const hex = accountGasLimits.slice(2)
		const verificationGasLimit = BigInt(`0x${hex.slice(0, 32)}`)
		const callGasLimit = BigInt(`0x${hex.slice(32, 64)}`)
		return { verificationGasLimit, callGasLimit }
	}

	/**
	 * Unpacks the `gasFees` bytes32 field back into its constituent fee values.
	 *
	 * @param gasFees - The packed 32-byte gas fees field from a `PackedUserOperation`.
	 * @returns Object with `maxPriorityFeePerGas` and `maxFeePerGas` as bigints.
	 */
	unpackGasFees(gasFees: HexString): { maxPriorityFeePerGas: bigint; maxFeePerGas: bigint } {
		const hex = gasFees.slice(2)
		const maxPriorityFeePerGas = BigInt(`0x${hex.slice(0, 32)}`)
		const maxFeePerGas = BigInt(`0x${hex.slice(32, 64)}`)
		return { maxPriorityFeePerGas, maxFeePerGas }
	}

	/**
	 * Converts a packed `PackedUserOperation` into the JSON object format
	 * expected by ERC-4337 bundler JSON-RPC endpoints.
	 *
	 * Unpacks `accountGasLimits` and `gasFees`, separates optional factory and
	 * paymaster fields, and converts all numeric fields to hex strings.
	 *
	 * @param userOp - The packed UserOperation to convert.
	 * @returns A plain object safe to pass as the first element of bundler RPC params.
	 */
	prepareBundlerCall(userOp: PackedUserOperation): Record<string, unknown> {
		const { verificationGasLimit, callGasLimit } = this.unpackGasLimits(userOp.accountGasLimits)
		const { maxPriorityFeePerGas, maxFeePerGas } = this.unpackGasFees(userOp.gasFees)

		const hasFactory = userOp.initCode && userOp.initCode !== "0x" && userOp.initCode.length > 2
		const factory = hasFactory ? (`0x${userOp.initCode.slice(2, 42)}` as HexString) : undefined
		const factoryData = hasFactory ? (`0x${userOp.initCode.slice(42)}` as HexString) : undefined

		const hasPaymaster =
			userOp.paymasterAndData && userOp.paymasterAndData !== "0x" && userOp.paymasterAndData.length > 2
		const paymaster = hasPaymaster ? (`0x${userOp.paymasterAndData.slice(2, 42)}` as HexString) : undefined
		const paymasterData = hasPaymaster ? (`0x${userOp.paymasterAndData.slice(42)}` as HexString) : undefined

		const userOpBundler: Record<string, unknown> = {
			sender: userOp.sender,
			nonce: toHex(userOp.nonce),
			callData: userOp.callData,
			callGasLimit: toHex(callGasLimit),
			verificationGasLimit: toHex(verificationGasLimit),
			preVerificationGas: toHex(userOp.preVerificationGas),
			maxFeePerGas: toHex(maxFeePerGas),
			maxPriorityFeePerGas: toHex(maxPriorityFeePerGas),
			signature: userOp.signature,
		}

		if (factory) {
			userOpBundler.factory = factory
			userOpBundler.factoryData = factoryData || "0x"
		}

		if (paymaster) {
			userOpBundler.paymaster = paymaster
			userOpBundler.paymasterData = paymasterData || "0x"
			userOpBundler.paymasterVerificationGasLimit = toHex(50_000n)
			userOpBundler.paymasterPostOpGasLimit = toHex(50_000n)
		}

		return userOpBundler
	}

	/**
	 * Sends a JSON-RPC request to the configured ERC-4337 bundler endpoint.
	 *
	 * @param method - The JSON-RPC method name (one of {@link BundlerMethod}).
	 * @param params - Array of parameters for the RPC call.
	 * @returns Resolves with the `result` field of the bundler's JSON-RPC response,
	 *   typed as `T`.
	 * @throws If the bundler URL is not configured or the bundler returns an error.
	 */
	async sendBundler<T = unknown>(method: BundlerMethod, params: unknown[] = []): Promise<T> {
		if (!this.ctx.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		const response = await fetch(this.ctx.bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
		})

		const result = await response.json()

		if (result.error) {
			throw new Error(`Bundler error: ${result.error.message || JSON.stringify(result.error)}`)
		}

		return result.result
	}

	/**
	 * Sends multiple JSON-RPC requests to the bundler in a single HTTP call
	 * using JSON-RPC 2.0 batch syntax.  Results are returned in the same order
	 * as the input `requests` array.
	 *
	 * @throws If the bundler URL is not configured, the HTTP call fails, or any
	 *   individual response contains an error.
	 */
	async sendBundlerBatch<T extends unknown[]>(
		requests: { method: BundlerMethod; params: unknown[] }[],
	): Promise<T> {
		if (!this.ctx.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		const body = requests.map((r, i) => ({
			jsonrpc: "2.0" as const,
			id: i + 1,
			method: r.method,
			params: r.params,
		}))

		const response = await fetch(this.ctx.bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(body),
		})

		const results = (await response.json()) as { id: number; result?: unknown; error?: { message?: string } }[]
		results.sort((a, b) => a.id - b.id)

		return results.map((r) => {
			if (r.error) {
				throw new Error(`Bundler error: ${r.error.message || JSON.stringify(r.error)}`)
			}
			return r.result
		}) as T
	}

	/**
	 * Encodes a list of calls into ERC-7821 `execute` calldata using
	 * single-batch mode (`ERC7821_BATCH_MODE`).
	 *
	 * @param calls - Ordered list of calls to batch; each specifies a `target`
	 *   address, ETH `value`, and `data`.
	 * @returns ABI-encoded calldata for the ERC-7821 `execute(bytes32,bytes)` function.
	 */
	encodeERC7821Execute(calls: ERC7821Call[]): HexString {
		const executionData = encodeAbiParameters(
			[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
			[calls.map((call) => ({ target: call.target, value: call.value, data: call.data }))],
		) as HexString

		return encodeFunctionData({
			abi: ERC7821ABI.ABI,
			functionName: "execute",
			args: [ERC7821_BATCH_MODE, executionData],
		}) as HexString
	}

	/**
	 * Decodes ERC-7821 `execute` calldata back into its constituent calls.
	 *
	 * Returns `null` if the calldata does not match the expected `execute`
	 * function signature or cannot be decoded.
	 *
	 * @param callData - Hex-encoded calldata previously produced by
	 *   {@link encodeERC7821Execute} or an equivalent encoder.
	 * @returns Array of decoded {@link ERC7821Call} objects, or `null` on failure.
	 */
	decodeERC7821Execute(callData: HexString): ERC7821Call[] | null {
		try {
			const decoded = decodeFunctionData({
				abi: ERC7821ABI.ABI,
				data: callData,
			})

			if (decoded?.functionName !== "execute" || !decoded.args || decoded.args.length < 2) {
				return null
			}

			const executionData = decoded.args[1] as HexString

			const [calls] = decodeAbiParameters(
				[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
				executionData,
			) as [ERC7821Call[]]

			return calls.map((call) => ({
				target: call.target as HexString,
				value: call.value,
				data: call.data as HexString,
			}))
		} catch {
			return null
		}
	}
}
