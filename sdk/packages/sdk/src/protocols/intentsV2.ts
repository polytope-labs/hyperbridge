import {
	encodeFunctionData,
	keccak256,
	toHex,
	encodeAbiParameters,
	concat,
	pad,
	maxUint256,
	type Hex,
	formatUnits,
	parseUnits,
} from "viem"
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts"
import IntentGatewayV2ABI from "@/abis/IntentGatewayV2"
import { createSessionKeyStorage, type SessionKeyData } from "@/storage"
import type {
	HexString,
	OrderV2,
	PackedUserOperation,
	SubmitBidOptions,
	EstimateFillOrderV2Params,
	FillOrderEstimateV2,
	IPostRequest,
	DispatchPost,
} from "@/types"
import type { SessionKeyStorageOptions } from "@/storage/types"
import {
	ADDRESS_ZERO,
	bytes32ToBytes20,
	bytes20ToBytes32,
	ERC20Method,
	retryPromise,
	fetchPrice,
	adjustDecimals,
	constructRedeemEscrowRequestBody,
	MOCK_ADDRESS,
	getRecordedStorageSlot,
} from "@/utils"
import { orderV2Commitment } from "@/utils"
import { Swap } from "@/utils/swap"
import { EvmChain } from "@/chains/evm"
import Decimal from "decimal.js"
import IntentGateway from "@/abis/IntentGateway"

/** EIP-712 type hash for SelectSolver message */
export const SELECT_SOLVER_TYPEHASH = keccak256(toHex("SelectSolver(bytes32 commitment,address solver)"))

/** Default graffiti value (bytes32 zero) */
export const DEFAULT_GRAFFITI = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

/**
 * IntentGatewayV2 utilities for placing orders and submitting bids.
 * Automatically manages session keys for solver selection.
 */
export class IntentGatewayV2 {
	private readonly storage: ReturnType<typeof createSessionKeyStorage>
	private readonly swap: Swap = new Swap()
	private readonly feeTokenCache: Map<string, { address: HexString; decimals: number }> = new Map()
	constructor(
		public readonly source: EvmChain,
		public readonly dest: EvmChain,
		storageOptions?: SessionKeyStorageOptions,
	) {
		this.storage = createSessionKeyStorage(storageOptions)
		this.initFeeTokenCache()
	}

	private async initFeeTokenCache(): Promise<void> {
		const sourceFeeToken = await this.source.getFeeTokenWithDecimals()
		this.feeTokenCache.set(this.source.config.stateMachineId, sourceFeeToken)
		const destFeeToken = await this.dest.getFeeTokenWithDecimals()
		this.feeTokenCache.set(this.dest.config.stateMachineId, destFeeToken)
	}

	// =========================================================================
	// Main Entry Points
	// =========================================================================

	/** Generates a session key, stores it, and returns encoded placeOrder calldata */
	async preparePlaceOrder(order: OrderV2, graffiti: HexString = DEFAULT_GRAFFITI): Promise<HexString> {
		const privateKey = generatePrivateKey()
		const account = privateKeyToAccount(privateKey)
		const sessionKeyAddress = account.address as HexString

		order.session = sessionKeyAddress

		const commitment = orderV2Commitment(order)

		const sessionKeyData: SessionKeyData = {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			commitment,
			createdAt: Date.now(),
		}
		await this.storage.setSessionKey(commitment, sessionKeyData)

		return encodeFunctionData({
			abi: IntentGatewayV2ABI.ABI,
			functionName: "placeOrder",
			args: [order, graffiti],
		}) as HexString
	}

	/** Prepares a bid UserOperation for submitting to Hyperbridge (used by fillers/solvers) */
	async prepareSubmitBid(options: SubmitBidOptions): Promise<PackedUserOperation> {
		const {
			order,
			fillOptions,
			solverAccount,
			solverPrivateKey,
			nonce,
			entryPointAddress,
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
		} = options

		const chainId = BigInt(
			this.dest.client.chain?.id ?? Number.parseInt(this.dest.config.stateMachineId.split("-")[1]),
		)

		const callData = encodeFunctionData({
			abi: IntentGatewayV2ABI.ABI,
			functionName: "fillOrder",
			args: [order, fillOptions],
		}) as HexString
		const commitment = orderV2Commitment(order)
		const accountGasLimits = this.packGasLimits(callGasLimit, verificationGasLimit)
		const gasFees = this.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

		const userOp: PackedUserOperation = {
			sender: solverAccount,
			nonce,
			initCode: "0x" as HexString,
			callData,
			accountGasLimits,
			preVerificationGas,
			gasFees,
			paymasterAndData: "0x" as HexString,
			signature: "0x" as HexString, // Will be signed later
		}

		const userOpHash = this.computeUserOpHash(userOp, entryPointAddress, chainId)
		const sessionKey = order.session

		// Sign: keccak256(abi.encodePacked(userOpHash, commitment, sessionKey))
		// sessionKey is address (20 bytes), not padded to 32
		const messageHash = keccak256(concat([userOpHash, commitment, sessionKey as Hex]))

		const solverAccount_ = privateKeyToAccount(solverPrivateKey as Hex)
		const solverSignature = await solverAccount_.signMessage({ message: { raw: messageHash } })

		// Signature: commitment (32 bytes) + solverSignature (65 bytes)
		const signature = concat([commitment, solverSignature as Hex]) as HexString

		return { ...userOp, signature }
	}

	/** Estimates gas costs for fillOrder execution via ERC-4337 */
	async estimateFillOrderV2(params: EstimateFillOrderV2Params): Promise<FillOrderEstimateV2> {
		const { order, solverAccountAddress } = params

		const totalEthValue = order.output.assets
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		const testValue = toHex(maxUint256 / 2n)
		const intentGatewayV2Address = this.dest.configService.getIntentGatewayV2Address(order.destination)
		const stateOverrides = this.buildTokenStateOverrides(
			this.dest.config.stateMachineId,
			order.output.assets,
			solverAccountAddress,
			this.dest.configService.getIntentGatewayAddress(order.destination),
			testValue,
			intentGatewayV2Address,
		)

		// Add native balance override for the solver account
		stateOverrides.push({
			address: solverAccountAddress,
			balance: maxUint256,
		})

		// Estimate fillOrder gas (callGasLimit)
		let callGasLimit: bigint
		const postRequestGas = 400_000n
		const sourceFeeToken = this.feeTokenCache.get(this.source.config.stateMachineId)!
		const destFeeToken = this.feeTokenCache.get(this.dest.config.stateMachineId)!
		const postRequestFeeInSourceFeeToken = await this.convertGasToFeeToken(
			postRequestGas as bigint,
			"source",
			params.order.source,
		)
		let postRequestFeeInDestFeeToken = adjustDecimals(
			postRequestFeeInSourceFeeToken,
			sourceFeeToken.decimals,
			destFeeToken.decimals,
		)

		const postRequest: IPostRequest = {
			source: params.order.destination,
			dest: params.order.source,
			body: constructRedeemEscrowRequestBody(
				{ ...params.order, id: orderV2Commitment(params.order) },
				MOCK_ADDRESS,
			),
			timeoutTimestamp: 0n,
			nonce: await this.source.getHostNonce(),
			from: this.source.configService.getIntentGatewayAddress(params.order.destination),
			to: this.source.configService.getIntentGatewayAddress(params.order.source),
		}

		let protocolFeeInNativeToken = await this.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() =>
			this.dest.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() => 0n),
		)

		// Buffer 0.5%
		protocolFeeInNativeToken = (protocolFeeInNativeToken * 1005n) / 1000n
		postRequestFeeInDestFeeToken = postRequestFeeInDestFeeToken + (postRequestFeeInDestFeeToken * 1005n) / 1000n

		if (!params.fillOptions) {
			params.fillOptions = {
				relayerFee: postRequestFeeInDestFeeToken,
				nativeDispatchFee: protocolFeeInNativeToken,
				outputs: order.output.assets,
			}
		}
		try {
			callGasLimit = await this.dest.client.estimateContractGas({
				abi: IntentGatewayV2ABI.ABI,
				address: this.dest.configService.getIntentGatewayV2Address(order.destination),
				functionName: "fillOrder",
				args: [order, params.fillOptions],
				account: solverAccountAddress,
				value: totalEthValue + protocolFeeInNativeToken,
				stateOverride: stateOverrides as any,
			})
		} catch (e) {
			console.warn("fillOrder gas estimation failed, using fallback:", e)
			callGasLimit = 500_000n
		}

		// Add buffer for execution through SolverAccount (5%)
		callGasLimit = callGasLimit + (callGasLimit * 5n) / 100n

		// Estimate verificationGasLimit for SolverAccount.validateUserOp
		const verificationGasLimit = 16_313n

		// Pre-verification gas (bundler overhead for calldata, etc.)
		const preVerificationGas = 21_000n

		// Get current gas prices
		const gasPrice = await this.dest.client.getGasPrice()
		const maxFeePerGas = gasPrice + (gasPrice * 20n) / 100n
		const maxPriorityFeePerGas = gasPrice / 10n

		// Calculate total gas cost in wei
		const totalGas = callGasLimit + verificationGasLimit + preVerificationGas
		const totalGasCostWei = totalGas * maxFeePerGas

		const totalGasInFeeToken = await this.convertGasToFeeToken(totalGasCostWei, "dest", order.destination)

		return {
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
			totalGasCostWei,
			totalGasInFeeToken,
			fillOptions: params.fillOptions,
		}
	}

	// =========================================================================
	// Signature & Hash Utilities
	// =========================================================================

	/** Signs a solver selection message using the stored session key (EIP-712) */
	async signSolverSelection(
		commitment: HexString,
		solverAddress: HexString,
		domainSeparator: HexString,
	): Promise<HexString | null> {
		const sessionKeyData = await this.storage.getSessionKey(commitment)
		if (!sessionKeyData) {
			return null
		}

		const account = privateKeyToAccount(sessionKeyData.privateKey as Hex)

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

	/** Computes the userOpHash for ERC-4337 v0.7 PackedUserOperation */
	computeUserOpHash(userOp: PackedUserOperation, entryPoint: HexString, chainId: bigint): HexString {
		const packedUserOp = encodeAbiParameters(
			[
				{ type: "address" },
				{ type: "uint256" },
				{ type: "bytes32" },
				{ type: "bytes32" },
				{ type: "bytes32" },
				{ type: "uint256" },
				{ type: "bytes32" },
				{ type: "bytes32" },
			],
			[
				userOp.sender,
				userOp.nonce,
				keccak256(userOp.initCode),
				keccak256(userOp.callData),
				userOp.accountGasLimits as Hex,
				userOp.preVerificationGas,
				userOp.gasFees as Hex,
				keccak256(userOp.paymasterAndData),
			],
		)

		const userOpHashInner = keccak256(packedUserOp)

		const outerEncoded = encodeAbiParameters(
			[{ type: "bytes32" }, { type: "address" }, { type: "uint256" }],
			[userOpHashInner, entryPoint, chainId],
		)

		return keccak256(outerEncoded)
	}

	// =========================================================================
	// Gas Packing Utilities
	// =========================================================================

	/** Packs callGasLimit and verificationGasLimit into bytes32 */
	packGasLimits(callGasLimit: bigint, verificationGasLimit: bigint): HexString {
		const callGasHex = pad(toHex(callGasLimit), { size: 16 })
		const verificationGasHex = pad(toHex(verificationGasLimit), { size: 16 })
		return concat([callGasHex, verificationGasHex]) as HexString
	}

	/** Packs maxPriorityFeePerGas and maxFeePerGas into bytes32 */
	packGasFees(maxPriorityFeePerGas: bigint, maxFeePerGas: bigint): HexString {
		const priorityFeeHex = pad(toHex(maxPriorityFeePerGas), { size: 16 })
		const maxFeeHex = pad(toHex(maxFeePerGas), { size: 16 })
		return concat([priorityFeeHex, maxFeeHex]) as HexString
	}

	// =========================================================================
	// Session Key Management
	// =========================================================================

	/** Retrieves a stored session key by order commitment */
	async getSessionKey(commitment: HexString): Promise<SessionKeyData | null> {
		return this.storage.getSessionKey(commitment)
	}

	/** Removes a stored session key */
	async removeSessionKey(commitment: HexString): Promise<void> {
		return this.storage.removeSessionKey(commitment)
	}

	/** Lists all stored session keys */
	async listSessionKeys(): Promise<SessionKeyData[]> {
		return this.storage.listSessionKeys()
	}

	// =========================================================================
	// Private Helpers
	// =========================================================================

	/** Builds state overrides for token balances and allowances to enable gas estimation */
	private buildTokenStateOverrides(
		chain: string,
		outputAssets: { token: HexString; amount: bigint }[],
		accountAddress: HexString,
		spenderAddress: HexString,
		testValue: HexString,
		intentGatewayV2Address?: HexString,
	): { address: HexString; balance?: bigint; stateDiff?: { slot: HexString; value: HexString }[] }[] {
		const overrides: { address: HexString; stateDiff: { slot: HexString; value: HexString }[] }[] = []

		// Params struct starts at slot 4, and slot 5 contains dispatcher + solverSelection packed
		// Slot 5 layout (64 hex chars after 0x):
		// - chars 2-23 (22 chars, 11 bytes): padding
		// - chars 24-25 (2 chars, 1 byte): solverSelection
		// - chars 26-65 (40 chars, 20 bytes): dispatcher
		if (intentGatewayV2Address) {
			const paramsSlot5 = pad(toHex(5n), { size: 32 }) as HexString
			const dispatcherAddress = this.dest.configService.getCalldispatcherAddress(chain)
			// Set solverSelection to 0x00, padding is zeros, dispatcher from config
			const newSlot5Value = ("0x" + "0".repeat(22) + "00" + dispatcherAddress.slice(2).toLowerCase()) as HexString
			overrides.push({
				address: intentGatewayV2Address,
				stateDiff: [{ slot: paramsSlot5, value: newSlot5Value }],
			})
		}

		for (const output of outputAssets) {
			const tokenAddress = bytes32ToBytes20(output.token)

			if (tokenAddress === ADDRESS_ZERO) {
				continue
			}

			try {
				const stateDiffs: { slot: HexString; value: HexString }[] = []

				const balanceData = (ERC20Method.BALANCE_OF + bytes20ToBytes32(accountAddress).slice(2)) as HexString
				const balanceSlot = getRecordedStorageSlot(chain, tokenAddress, balanceData)
				if (balanceSlot) {
					stateDiffs.push({ slot: balanceSlot, value: testValue })
				}

				try {
					const allowanceData = (ERC20Method.ALLOWANCE +
						bytes20ToBytes32(accountAddress).slice(2) +
						bytes20ToBytes32(spenderAddress).slice(2)) as HexString
					const allowanceSlot = getRecordedStorageSlot(chain, tokenAddress, allowanceData)
					if (allowanceSlot) {
						stateDiffs.push({ slot: allowanceSlot, value: testValue })
					}
				} catch (e) {
					console.warn(`Could not find allowance slot for token ${tokenAddress}:`, e)
				}

				overrides.push({ address: tokenAddress, stateDiff: stateDiffs })
			} catch (e) {
				console.warn(`Could not find balance slot for token ${tokenAddress}:`, e)
			}
		}

		return overrides
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token (DAI).
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 *
	 * @param gasEstimate - The estimated gas units
	 * @param gasEstimateIn - Whether to use "source" or "dest" chain for the conversion
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @returns The gas cost converted to fee token amount
	 * @private
	 */
	private async convertGasToFeeToken(
		gasEstimate: bigint,
		gasEstimateIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[gasEstimateIn].client
		const gasPrice = await retryPromise(() => client.getGasPrice(), { maxRetries: 3, backoffMs: 250 })
		const gasCostInWei = gasEstimate * gasPrice
		const wethAddr = this[gasEstimateIn].configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = this.feeTokenCache.get(evmChainID)!

		try {
			const { amountOut } = await this.swap.findBestProtocolWithAmountIn(
				this[gasEstimateIn].client,
				wethAddr,
				feeToken.address,
				gasCostInWei,
				evmChainID,
				{ selectedProtocol: "v2" },
			)
			if (amountOut === 0n) {
				console.log("Amount out not found")
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
			const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeCurrency?.decimals!))
			const tokenPriceUsd = await fetchPrice(nativeCurrency?.symbol!, chainId)
			const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
			const feeTokenPriceUsd = new Decimal(1) // stable coin
			const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
			return parseUnits(gasCostInFeeToken.toFixed(feeToken.decimals), feeToken.decimals)
		}
	}

	/**
	 * Gets a quote for the native token cost of dispatching a post request.
	 *
	 * @param postRequest - The post request to quote
	 * @param fee - The fee amount in fee token
	 * @returns The native token amount required
	 */
	private async quoteNative(postRequest: IPostRequest, fee: bigint): Promise<bigint> {
		const dispatchPost: DispatchPost = {
			dest: toHex(postRequest.dest),
			to: postRequest.to,
			body: postRequest.body,
			timeout: postRequest.timeoutTimestamp,
			fee: fee,
			payer: postRequest.from,
		}

		const quoteNative = await this.dest.client.readContract({
			address: this.dest.configService.getIntentGatewayAddress(postRequest.dest),
			abi: IntentGateway.ABI,
			functionName: "quoteNative",
			args: [dispatchPost] as any,
		})

		return quoteNative
	}
}
