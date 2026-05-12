import type { Address } from "viem"
import { encodeFunctionData, decodeEventLog, encodeAbiParameters, toHex, type Hex } from "viem"
import { EvmChain } from "@/chains/evm"
import { IsmpClient } from "@/client"
import { HyperFungibleTokenABI, WrappedHyperFungibleTokenABI } from "@/abis/hyperFungibleToken"
import { ERC20ABI } from "@/abis/erc20"
import type { HexString, IPostRequest, RequestStatusWithMetadata } from "@/types"

/** IWrappedHyperFungibleToken ERC165 interface ID */
const WRAPPED_HFT_INTERFACE_ID = "0xe23d9765" as HexString

/**
 * Human-readable parameters for bridging tokens cross-chain
 */
export interface BridgeParams {
	/** Address of the HFT or WrappedHFT contract on the source chain */
	token: Address
	/** The sender's address (needed for allowance checks and fee quoting) */
	from: Address
	/** Recipient on the destination chain (20 bytes for EVM, 32 bytes for substrate) */
	to: HexString | Uint8Array
	/** Amount of tokens to send (in token decimals) */
	amount: bigint
	/** Destination state machine ID (e.g. "EVM-97") */
	dest: string | Uint8Array
	/** Request timeout in seconds (default: 3600) */
	timeout?: bigint
	/** Optional calldata to execute on the destination chain */
	data?: HexString
	/** Pay fees in the host's feeToken instead of native. Default: false (pay native) */
	payInFeeToken?: boolean
	/** Relayer fee override in feeToken units. If not set, automatically estimated from destination gas cost. Set to 0n for self-relay. */
	relayerFee?: bigint
}

/**
 * Fee quote for a cross-chain send
 */
export interface QuoteResult {
	/** Native token amount needed as msg.value for send() (protocol fee + relayer fee) */
	totalNativeCost: bigint
	/** Total fee in the host's fee token (protocol fee + relayer fee) */
	totalFeeTokenCost: bigint
	/** Relayer fee component in the source chain's fee token */
	relayerFeeInFeeToken: bigint
}

/**
 * Steps yielded by the bridge generator
 */
export type BridgeStep =
	| { type: "approve"; tx: { to: Address; data: HexString } }
	| { type: "send"; tx: { to: Address; data: HexString; value: bigint } }
	| { type: "submitted"; commitment: HexString }
	| ({ type: "status" } & RequestStatusWithMetadata)

/**
 * SDK class for cross-chain token transfers via HyperFungibleToken contracts.
 *
 * Supports both HyperFungibleToken (burn/mint) and WrappedHyperFungibleToken (lock/unlock)
 * with automatic type detection via ERC165.
 *
 * @example
 * ```typescript
 * const hft = new HyperFungibleToken({ source, dest })
 *
 * // Quote the fee
 * const fee = await hft.quote({ token, from, to, amount, dest: "EVM-97" })
 *
 * // Bridge tokens
 * const gen = hft.bridge({ token, from, to, amount, dest: "EVM-97" })
 * for await (const step of gen) {
 *   if (step.type === "approve" || step.type === "send") {
 *     const hash = await walletClient.sendTransaction(step.tx)
 *     await gen.next(hash) // resume with tx hash
 *   }
 * }
 * ```
 */
export class HyperFungibleToken {
	private readonly source: EvmChain
	private readonly dest: EvmChain
	private readonly client?: IsmpClient

	/**
	 * @param params.source - Source EVM chain where tokens are sent from
	 * @param params.dest - Destination EVM chain where tokens are received
	 * @param params.client - Optional ISMP client for tracking request status after submission.
	 *   If not provided, the bridge generator terminates at the "submitted" step.
	 */
	constructor(params: { source: EvmChain; dest: EvmChain; client?: IsmpClient }) {
		this.source = params.source
		this.dest = params.dest
		this.client = params.client
	}

	/**
	 * Detects whether a token contract is a WrappedHyperFungibleToken via ERC165.
	 */
	async isWrapped(tokenAddress: Address): Promise<boolean> {
		try {
			return (await this.source.client.readContract({
				address: tokenAddress,
				abi: HyperFungibleTokenABI,
				functionName: "supportsInterface",
				args: [WRAPPED_HFT_INTERFACE_ID],
			})) as boolean
		} catch {
			return false
		}
	}

	/**
	 * Quotes the fee for a cross-chain send.
	 *
	 * 1. Estimates gas for delivering the message on the destination chain (fake proofs)
	 * 2. Converts dest gas cost → dest feeToken via dest chain's Uniswap (getAmountsOut)
	 * 3. Scales decimals if source and dest feeTokens differ in precision
	 * 4. Passes relayer fee (in source feeToken) to source.quoteNative() which adds
	 *    per-byte protocol fees and converts the total to source native
	 */
	async quote(params: BridgeParams): Promise<QuoteResult> {
		const dest = this.encodeStateMachineId(params.dest)
		const to: Hex = typeof params.to === "string" ? (params.to as Hex) : toHex(params.to)
		const data = (params.data ?? "0x") as Hex
		const timeout = params.timeout ?? 3600n
		const destChainId =
			typeof params.dest === "string" ? params.dest : new TextDecoder().decode(params.dest)
		const sourceChainId = this.source.config.stateMachineId

		// Build the message body as the contract would encode it
		const messageBody = encodeAbiParameters(
			[
				{ name: "from", type: "bytes" },
				{ name: "to", type: "bytes" },
				{ name: "amount", type: "uint256" },
				{ name: "data", type: "bytes" },
			],
			[toHex(params.from) as Hex, to, params.amount, data],
		)

		// Batch 1: independent queries in parallel
		const [peerModuleId, sourceFeeToken, destFeeToken, gasPrice] = await Promise.all([
			this.source.client.readContract({
				address: params.token,
				abi: HyperFungibleTokenABI,
				functionName: "supportedChain",
				args: [dest],
			}) as Promise<Hex>,
			this.source.getFeeTokenWithDecimals(),
			this.dest.getFeeTokenWithDecimals(),
			this.dest.client.getGasPrice(),
		])

		const postRequest: IPostRequest = {
			source: sourceChainId,
			dest: destChainId,
			from: params.token,
			to: peerModuleId,
			nonce: 0n,
			body: messageBody,
			timeoutTimestamp: timeout,
		}

		let relayerFeeInSourceFeeToken: bigint

		if (params.relayerFee !== undefined) {
			relayerFeeInSourceFeeToken = params.relayerFee
		} else {
			const minRelayerFee = (5n * 10n ** BigInt(sourceFeeToken.decimals)) / 100n
			relayerFeeInSourceFeeToken = minRelayerFee

			try {
				const { gas } = await this.dest.estimateGas(postRequest)
				const gasCostInDestNative = (gas * gasPrice * 110n) / 100n

				const relayerFeeInDestFeeToken = await this.dest.getAmountsOut(
					gasCostInDestNative,
					destFeeToken.address,
				)

				let scaledFee = relayerFeeInDestFeeToken
				if (sourceFeeToken.decimals > destFeeToken.decimals) {
					scaledFee = relayerFeeInDestFeeToken * 10n ** BigInt(sourceFeeToken.decimals - destFeeToken.decimals)
				} else if (sourceFeeToken.decimals < destFeeToken.decimals) {
					scaledFee = relayerFeeInDestFeeToken / 10n ** BigInt(destFeeToken.decimals - sourceFeeToken.decimals)
				}

				if (scaledFee > relayerFeeInSourceFeeToken) {
					relayerFeeInSourceFeeToken = scaledFee
				}
			} catch {
				// Gas estimation or swap quote failed — use minimum
			}
		}

		// Batch 3: call quote and quoteNative with SendParams on the token contract
		const sendParams = {
			dest,
			to,
			amount: params.amount,
			timeout,
			relayerFee: relayerFeeInSourceFeeToken,
			data,
		}

		// quote() always works; quoteNative() may fail if no Uniswap router
		const totalFeeTokenCost = (await this.source.client.readContract({
			address: params.token,
			abi: HyperFungibleTokenABI,
			functionName: "quote",
			args: [sendParams],
		})) as bigint

		let totalNativeCost = 0n
		try {
			totalNativeCost = (await this.source.client.readContract({
				address: params.token,
				abi: HyperFungibleTokenABI,
				functionName: "quoteNative",
				args: [sendParams],
			})) as bigint
			totalNativeCost = (totalNativeCost * 101n) / 100n // 1% buffer
		} catch {
			// No Uniswap router — native quote unavailable, use feeToken path
		}

		return {
			totalNativeCost,
			totalFeeTokenCost,
			relayerFeeInFeeToken: relayerFeeInSourceFeeToken,
		}
	}

	/**
	 * Generator-based bridge flow for cross-chain token transfers.
	 *
	 * Yields steps that the caller must execute:
	 * 1. `approve` — ERC20 approval tx (only for WrappedHFT with insufficient allowance)
	 * 2. `send` — the cross-chain send tx
	 * 3. `submitted` — commitment hash after tx is mined
	 * 4. `status` — ISMP request status updates (only if ismpClient was provided)
	 *
	 * Resume the generator with the tx hash after submitting each tx.
	 */
	async *bridge(params: BridgeParams): AsyncGenerator<BridgeStep, void, HexString | undefined> {
		const token = params.token
		const dest = this.encodeStateMachineId(params.dest)
		const to: Hex = typeof params.to === "string" ? (params.to as Hex) : toHex(params.to)
		const timeout = params.timeout ?? 3600n
		const data = (params.data ?? "0x") as Hex
		const payInFeeToken = params.payInFeeToken ?? false

		// Batch 1: quote + type detection + feeToken lookup (all independent)
		const [fee, wrapped, sourceFeeToken] = await Promise.all([
			this.quote(params),
			this.isWrapped(token),
			payInFeeToken ? this.source.getFeeTokenWithDecimals() : Promise.resolve(null),
		])

		// Batch 2: wrapped token checks (conditional)
		const wrappedResult = wrapped
			? await Promise.all([
					this.source.client.readContract({
						address: token,
						abi: WrappedHyperFungibleTokenABI,
						functionName: "isWeth",
					}) as Promise<boolean>,
					this.source.client.readContract({
						address: token,
						abi: WrappedHyperFungibleTokenABI,
						functionName: "underlying",
					}) as Promise<Address>,
				])
			: null

		// Step 1: Underlying ERC20 approval (wrapped, non-WETH only)
		if (wrappedResult) {
			const [isWeth, underlying] = wrappedResult
			if (!isWeth) {
				const currentAllowance = (await this.source.client.readContract({
					address: underlying,
					abi: ERC20ABI,
					functionName: "allowance",
					args: [params.from, token],
				})) as bigint

				if (currentAllowance < params.amount) {
					yield {
						type: "approve",
						tx: {
							to: underlying,
							data: encodeFunctionData({
								abi: ERC20ABI,
								functionName: "approve",
								args: [token, params.amount],
							}) as HexString,
						},
					}
				}
			}
		}

		// Step 2: FeeToken approval (payInFeeToken only)
		if (payInFeeToken && sourceFeeToken) {
			const feeTokenAddress = sourceFeeToken.address as Address
			const currentAllowance = (await this.source.client.readContract({
				address: feeTokenAddress,
				abi: ERC20ABI,
				functionName: "allowance",
				args: [params.from, token],
			})) as bigint

			// Approve max to avoid repeated approvals and rounding issues
			if (currentAllowance < fee.totalFeeTokenCost) {
				yield {
					type: "approve",
					tx: {
						to: feeTokenAddress,
						data: encodeFunctionData({
							abi: ERC20ABI,
							functionName: "approve",
							args: [token, 2n ** 256n - 1n], // max approval
						}) as HexString,
					},
				}
			}
		}

		// Step 3: Build and yield send tx
		const isWeth = wrappedResult?.[0] ?? false
		const sendData = encodeFunctionData({
			abi: HyperFungibleTokenABI,
			functionName: "send",
			args: [{ dest, to, amount: params.amount, timeout, relayerFee: fee.relayerFeeInFeeToken, data }],
		})

		// For WETH wrappers: msg.value must include the amount being wrapped
		// plus native fee payment (or just the amount if paying fees in fee token)
		let value: bigint
		if (isWeth) {
			value = payInFeeToken ? params.amount : params.amount + fee.totalNativeCost
		} else {
			value = payInFeeToken ? 0n : fee.totalNativeCost
		}

		const txHash: HexString | undefined = yield {
			type: "send",
			tx: {
				to: token,
				data: sendData as HexString,
				value,
			},
		}

		if (!txHash) return

		// Step 3: Read tx receipt and extract commitment from Sent event
		const receipt = await this.source.client.waitForTransactionReceipt({
			hash: txHash as Hex,
		})

		let commitment: HexString | undefined
		for (const log of receipt.logs) {
			try {
				const decoded = decodeEventLog({
					abi: HyperFungibleTokenABI,
					data: log.data,
					topics: log.topics,
				})
				if (decoded.eventName === "Sent") {
					commitment = (decoded.args as any).commitment as HexString
					break
				}
			} catch {
				// Not our event
			}
		}

		if (!commitment) return

		yield { type: "submitted", commitment }

		// Step 4: Stream ISMP status if client provided
		if (this.client) {
			for await (const update of this.client.postRequestStatusStream(commitment)) {
				yield { type: "status", ...update }
				if (update.status === "DESTINATION" || update.status === "PENDING_TIMEOUT") {
					break
				}
			}
		}
	}

	/**
	 * Encodes a state machine ID string to hex bytes.
	 */
	private encodeStateMachineId(dest: string | Uint8Array): Hex {
		if (typeof dest === "string") {
			return toHex(new TextEncoder().encode(dest))
		}
		return toHex(dest)
	}
}
