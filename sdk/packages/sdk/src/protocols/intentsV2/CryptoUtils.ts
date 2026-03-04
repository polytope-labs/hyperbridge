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
import type { IntentsV2Context } from "./types"
import { ERC7821_BATCH_MODE } from "./types"
import type { BundlerMethod } from "./types"

/** EIP-712 type hash for SelectSolver message */
export const SELECT_SOLVER_TYPEHASH = keccak256(toHex("SelectSolver(bytes32 commitment,address solver)"))

/** EIP-712 type hash for PackedUserOperation */
export const PACKED_USEROP_TYPEHASH = keccak256(
	toHex(
		"PackedUserOperation(address sender,uint256 nonce,bytes initCode,bytes callData,bytes32 accountGasLimits,uint256 preVerificationGas,bytes32 gasFees,bytes paymasterAndData)",
	),
)

/** EIP-712 type hash for EIP712Domain */
export const DOMAIN_TYPEHASH = keccak256(
	toHex("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
)

/**
 * Crypto and encoding utilities for IntentGatewayV2: EIP-712, UserOp hashing,
 * gas packing, bundler calls, and ERC-7821 encode/decode.
 */
export class CryptoUtils {
	constructor(private readonly ctx: IntentsV2Context) {}

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

	computeUserOpHash(userOp: PackedUserOperation, entryPoint: Hex, chainId: bigint): Hex {
		const structHash = this.getPackedUserStructHash(userOp)
		const domainSeparator = this.getDomainSeparator("ERC4337", "1", chainId, entryPoint as HexString)

		return keccak256(
			encodePacked(["bytes1", "bytes1", "bytes32", "bytes32"], ["0x19", "0x01", domainSeparator, structHash]),
		)
	}

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

	packGasLimits(verificationGasLimit: bigint, callGasLimit: bigint): HexString {
		const verificationGasHex = pad(toHex(verificationGasLimit), { size: 16 })
		const callGasHex = pad(toHex(callGasLimit), { size: 16 })
		return concat([verificationGasHex, callGasHex]) as HexString
	}

	packGasFees(maxPriorityFeePerGas: bigint, maxFeePerGas: bigint): HexString {
		const priorityFeeHex = pad(toHex(maxPriorityFeePerGas), { size: 16 })
		const maxFeeHex = pad(toHex(maxFeePerGas), { size: 16 })
		return concat([priorityFeeHex, maxFeeHex]) as HexString
	}

	unpackGasLimits(accountGasLimits: HexString): { verificationGasLimit: bigint; callGasLimit: bigint } {
		const hex = accountGasLimits.slice(2)
		const verificationGasLimit = BigInt(`0x${hex.slice(0, 32)}`)
		const callGasLimit = BigInt(`0x${hex.slice(32, 64)}`)
		return { verificationGasLimit, callGasLimit }
	}

	unpackGasFees(gasFees: HexString): { maxPriorityFeePerGas: bigint; maxFeePerGas: bigint } {
		const hex = gasFees.slice(2)
		const maxPriorityFeePerGas = BigInt(`0x${hex.slice(0, 32)}`)
		const maxFeePerGas = BigInt(`0x${hex.slice(32, 64)}`)
		return { maxPriorityFeePerGas, maxFeePerGas }
	}

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
