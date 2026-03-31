import type { Address } from "viem"
import { toHex, encodeAbiParameters, parseAbiParameters } from "viem"
import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import UniswapRouterV2 from "@/abis/uniswapRouterV2"
import type { HexString, DispatchPost, IPostRequest } from "@/types"

/**
 * Result of the quoteNative fee estimation
 */
export interface QuoteNativeResult {
	/** Total native token cost including relayer fee and protocol fee with 1% buffer */
	totalNativeCost: bigint
	/** Relayer fee converted to source chain fee token */
	relayerFeeInSourceFeeToken: bigint
}

/**
 * Parameters for token gateway teleport operations
 */
export interface TeleportParams {
	/** Amount to be sent */
	amount: bigint
	/** The token identifier to send */
	assetId: HexString
	/** Redeem ERC20 on the destination? */
	redeem: boolean
	/** Recipient address */
	to: HexString
	/** Recipient state machine */
	dest: string | Uint8Array
	/** Request timeout in seconds */
	timeout: bigint
	/** Destination contract call data */
	data?: HexString | Uint8Array
}

/**
 * TokenGateway class for managing cross-chain token transfers via Hyperbridge
 *
 * This class provides methods to interact with the TokenGateway contract, including
 * estimating fees for cross-chain token teleports.
 *
 * Supports both EVM and Substrate chains as destination.
 *
 * @example
 * ```typescript
 * const tokenGateway = new TokenGateway({
 *   source: sourceChain,
 *   dest: destChain // Can be EvmChain or SubstrateChain
 * })
 *
 * const teleportParams: TeleportParams = {
 *   amount: parseEther("1.0"),
 *   assetId: keccak256(toHex("USDC")),
 *   redeem: true,
 *   to: pad("0xRecipientAddress", { size: 32 }),
 *   dest: "EVM-1",
 *   timeout: 3600n,
 * }
 *
 * // Estimate native cost (relayer fee + protocol fee with 1% buffer)
 * const { totalNativeCost, relayerFeeInSourceFeeToken } = await tokenGateway.quoteNative(teleportParams)
 * console.log(`Total native cost: ${formatEther(totalNativeCost)} ETH`)
 * console.log(`Relayer fee in fee token: ${relayerFeeInSourceFeeToken}`)
 * ```
 */
export class TokenGateway {
	private readonly source: EvmChain
	private readonly dest: EvmChain | SubstrateChain

	constructor(params: { source: EvmChain; dest: EvmChain | SubstrateChain }) {
		this.source = params.source
		this.dest = params.dest
	}

	/**
	 * Get the TokenGateway contract address for a given chain
	 *
	 * @param chain - The chain identifier (e.g., "EVM-1", "EVM-56")
	 * @returns The TokenGateway contract address
	 */
	private getTokenGatewayAddress(chain: string | Uint8Array): Address {
		const chainStr = typeof chain === "string" ? chain : new TextDecoder().decode(chain)
		return this.source.configService.getTokenGatewayAddress(chainStr)
	}

	/**
	 * Estimate the native token cost for a token gateway teleport operation.
	 * This includes both relayer fees and protocol fees for cross-chain delivery.
	 *
	 * The relayer fee is automatically estimated for EVM destination chains by:
	 * 1. Creating a dummy post request with 191 bytes of random data in the body
	 * 2. Estimating gas for delivery on the destination chain
	 * 3. Converting the gas estimate to native tokens
	 * 4. Adding a 1% buffer to the relayer fee for safety margin
	 *
	 * For non-EVM destination chains, the relayer fee is set to zero.
	 *
	 * The function then constructs a proper post request and calls quoteNative on the
	 * source chain to get protocol fees (with 1% buffer), converts the relayer fee to
	 * source chain fee token using Uniswap V2's getAmountsOut, and returns both values.
	 *
	 * @param params - The teleport parameters
	 * @returns Object containing totalNativeCost (with 1% buffer) and relayerFeeInSourceFeeToken
	 *
	 * @throws Will throw an error if the contract call fails
	 *
	 * @example
	 * ```typescript
	 * const params: TeleportParams = {
	 *   amount: parseEther("1.0"),
	 *   assetId: keccak256(toHex("USDC")),
	 *   redeem: true,
	 *   to: pad("0xRecipientAddress", { size: 32 }),
	 *   dest: "EVM-1",
	 *   timeout: 3600n,
	 *   data: "0x"
	 * }
	 *
	 * const { totalNativeCost, relayerFeeInSourceFeeToken } = await tokenGateway.quoteNative(params)
	 * console.log(`Total native cost: ${formatEther(totalNativeCost)} ETH`)
	 * console.log(`Relayer fee in fee token: ${relayerFeeInSourceFeeToken}`)
	 * ```
	 */
	async quoteNative(params: TeleportParams): Promise<QuoteNativeResult> {
		// Convert data to hex if it's Uint8Array, default to empty bytes
		const dataHex = params.data ? (typeof params.data === "string" ? params.data : toHex(params.data)) : "0x"

		// Get the TokenGateway addresses
		const sourceTokenGatewayAddress = this.getTokenGatewayAddress(this.source.config.stateMachineId)
		const destTokenGatewayAddress = this.getTokenGatewayAddress(params.dest)

		let relayerFee = 0n

		// Only estimate relayer fee if destination is an EVM chain
		const destChainId = typeof params.dest === "string" ? params.dest : new TextDecoder().decode(params.dest)
		const isEvmDest = destChainId.startsWith("EVM-") && this.dest instanceof EvmChain

		if (isEvmDest) {
			// Create a dummy post request with 191 bytes of random data
			// Generate 191 random bytes as hex string (191 * 2 hex chars + 0x prefix)
			const randomHex =
				"0x" + Array.from({ length: 191 * 2 }, () => Math.floor(Math.random() * 16).toString(16)).join("")
			const randomBody = randomHex as HexString

			const dummyPostRequest: IPostRequest = {
				source: this.source.config.stateMachineId,
				dest: destChainId,
				from: sourceTokenGatewayAddress,
				to: destTokenGatewayAddress,
				nonce: 0n,
				body: randomBody,
				timeoutTimestamp: params.timeout,
			}

			// Estimate gas on destination chain (only available for EvmChain)
			const { gas } = await (this.dest as EvmChain).estimateGas(dummyPostRequest)

			// Get current gas price on destination chain
			const gasPrice = await (this.dest as EvmChain).client.getGasPrice()

			// Calculate gas cost in native tokens (gas * gasPrice)
			const gasCostInNative = gas * gasPrice

			// Add 1% buffer to relayer fee
			relayerFee = (gasCostInNative * 101n) / 100n
		}

		// Now encode the actual teleport body with the calculated relayer fee
		const teleportBody = encodeAbiParameters(
			parseAbiParameters("uint256, uint256, bytes32, bool, bytes32, bytes"),
			[
				params.amount,
				relayerFee, // Use the calculated relayer fee (0 for non-EVM destinations)
				params.assetId,
				params.redeem,
				params.to,
				dataHex as `0x${string}`,
			],
		)

		// Create the actual post request for protocol fee estimation
		const postRequest: IPostRequest = {
			source: this.source.config.stateMachineId,
			dest: destChainId,
			from: sourceTokenGatewayAddress,
			to: destTokenGatewayAddress,
			nonce: 0n,
			body: teleportBody,
			timeoutTimestamp: params.timeout,
		}

		// Get protocol fee from source chain by calling quoteNative
		// This returns the cost in native tokens for dispatching the request
		const protocolFeeInNative = await this.source.quoteNative(postRequest, relayerFee)

		// Add 1% buffer to the protocol fee
		const protocolFeeWithBuffer = (protocolFeeInNative * 101n) / 100n

		// Convert relayer fee from native to source fee token
		let relayerFeeInSourceFeeToken = 0n
		if (relayerFee > 0n) {
			// Get fee token details from source chain
			const feeToken = await this.source.getFeeTokenWithDecimals()

			// Convert native relayer fee to fee token using Uniswap
			relayerFeeInSourceFeeToken = await this.convertNativeToFeeToken(
				relayerFee,
				feeToken.address,
				this.source.config.stateMachineId,
			)
		}

		return {
			totalNativeCost: protocolFeeWithBuffer,
			relayerFeeInSourceFeeToken,
		}
	}

	/**
	 * Convert native token amount to fee token amount using Uniswap V2 router
	 * @private
	 */
	private async convertNativeToFeeToken(
		nativeAmount: bigint,
		feeTokenAddress: HexString,
		chain: string,
	): Promise<bigint> {
		const v2Router = this.source.configService.getUniswapRouterV2Address(chain)
		const WETH = this.source.configService.getWrappedNativeAssetWithDecimals(chain).asset

		const v2AmountOut = await this.source.client.simulateContract({
			address: v2Router,
			abi: UniswapRouterV2.ABI,
			// @ts-ignore
			functionName: "getAmountsOut",
			// @ts-ignore
			args: [nativeAmount, [WETH, feeTokenAddress]],
		})

		return v2AmountOut.result[1]
	}
}
