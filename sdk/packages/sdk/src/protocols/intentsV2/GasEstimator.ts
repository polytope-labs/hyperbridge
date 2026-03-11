import { encodeFunctionData, toHex, pad, maxUint256, concat, keccak256, isHex, hexToString } from "viem"
import { generatePrivateKey, privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import IntentGateway from "@/abis/IntentGateway"
import {
	ADDRESS_ZERO,
	bytes32ToBytes20,
	bytes20ToBytes32,
	ERC20Method,
	adjustDecimals,
	constructRedeemEscrowRequestBody,
	MOCK_ADDRESS,
	getOrFetchStorageSlot,
	EvmLanguage,
} from "@/utils"
import { orderV2Commitment } from "@/utils"
import { calculateBalanceMappingLocation } from "@/utils"
import type {
	OrderV2,
	PackedUserOperation,
	EstimateFillOrderV2Params,
	FillOrderEstimateV2,
	FillOptionsV2,
	IPostRequest,
	DispatchPost,
} from "@/types"
import type { HexString } from "@/types"
import type { IntentsV2Context } from "./types"
import { BundlerMethod } from "./types"
import type { BundlerGasEstimate, PimlicoGasPriceEstimate } from "./types"
import { getFeeToken, transformOrderForContract, convertGasToFeeToken } from "./utils"
import { CryptoUtils } from "./CryptoUtils"

/**
 * Estimates the gas cost for filling an IntentGatewayV2 order and converts it
 * into the source-chain fee token so callers can set `order.fees` accurately.
 *
 * When a bundler URL is configured, estimation uses
 * `eth_estimateUserOperationGas` with realistic state overrides (token
 * balances, allowances, EntryPoint deposits, and optional solver account
 * bytecode). Without a bundler, it falls back to `estimateContractGas`.
 * Pimlico-specific gas-price refinement is applied automatically when the
 * bundler URL contains `pimlico.io`.
 */
export class GasEstimator {
	/**
	 * @param ctx - Shared IntentsV2 context providing the source and destination
	 *   chain clients, config service, bundler URL, and solver-code cache.
	 * @param crypto - Crypto utilities used for UserOp construction, signing,
	 *   gas packing, and bundler calls.
	 */
	constructor(
		private readonly ctx: IntentsV2Context,
		private readonly crypto: CryptoUtils,
	) {}

	/**
	 * Estimates the gas cost for a solver to fill the given order and returns
	 * a structured estimate with individual gas components and total costs in
	 * both wei and fee-token units.
	 *
	 * **Cross-chain orders:** also estimates the ISMP POST request fee required
	 * for the solver to trigger source-chain escrow redemption after filling, and
	 * includes it in `fillOptions.relayerFee` and `fillOptions.nativeDispatchFee`.
	 *
	 * **Bundler path:** constructs a mock `PackedUserOperation` signed by an
	 * ephemeral keypair, applies state overrides, and calls
	 * `eth_estimateUserOperationGas`. Gas limits are bumped by 5-10% for
	 * headroom. If the bundler is Pimlico, gas prices are refined with
	 * `pimlico_getUserOperationGasPrice`.
	 *
	 * **Fallback path (no bundler):** calls `estimateContractGas` directly on
	 * `fillOrder` with state overrides.
	 *
	 * @param params - Parameters including the order to estimate and optional
	 *   percentage bumps for `maxPriorityFeePerGas` and `maxFeePerGas`.
	 * @returns A {@link FillOrderEstimateV2} containing all gas components,
	 *   EIP-1559 fee values, total cost in wei, and total cost in the source
	 *   chain's fee token.
	 */
	async estimateFillOrderV2(params: EstimateFillOrderV2Params): Promise<FillOrderEstimateV2> {
		const { order } = params
		const solverPrivateKey = generatePrivateKey()
		const solverAccountAddress = privateKeyToAddress(solverPrivateKey)
		const souceStateMachineId = isHex(order.source) ? hexToString(order.source) : order.source
		const destStateMachineId = isHex(order.destination) ? hexToString(order.destination) : order.destination
		const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayV2Address(destStateMachineId)
		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(destStateMachineId)
		const chainId = BigInt(Number.parseInt(destStateMachineId.split("-")[1]))

		const totalEthValue = order.output.assets
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		const sourceFeeToken = await getFeeToken(this.ctx, this.ctx.source.config.stateMachineId, this.ctx.source)
		const destFeeToken = await getFeeToken(this.ctx, this.ctx.dest.config.stateMachineId, this.ctx.dest)
		const feeTokenAsBytes32 = bytes20ToBytes32(destFeeToken.address)
		const assetsForOverrides = [...order.output.assets]
		if (!assetsForOverrides.some((asset) => asset.token.toLowerCase() === feeTokenAsBytes32.toLowerCase())) {
			assetsForOverrides.push({ token: feeTokenAsBytes32, amount: 0n })
		}

		const { viem: stateOverrides, bundler: bundlerStateOverrides } = await this.buildStateOverride({
			accountAddress: solverAccountAddress,
			chain: destStateMachineId,
			outputAssets: assetsForOverrides,
			spenderAddress: intentGatewayV2Address,
			intentGatewayV2Address,
			entryPointAddress,
		})

		const isSameChain = souceStateMachineId === destStateMachineId
		let postRequestFeeInDestFeeToken = 0n
		let protocolFeeInNativeToken = 0n

		if (!isSameChain) {
			const postRequestGas = 400_000n
			const postRequestFeeInSourceFeeToken = await convertGasToFeeToken(
				this.ctx,
				postRequestGas,
				"source",
				souceStateMachineId,
			)
			postRequestFeeInDestFeeToken = adjustDecimals(
				postRequestFeeInSourceFeeToken,
				sourceFeeToken.decimals,
				destFeeToken.decimals,
			)

			const postRequest: IPostRequest = {
				source: destStateMachineId,
				dest: souceStateMachineId,
				body: constructRedeemEscrowRequestBody({ ...order, id: orderV2Commitment(order) }, MOCK_ADDRESS),
				timeoutTimestamp: 0n,
				nonce: await this.ctx.dest.getHostNonce(),
				from: this.ctx.source.configService.getIntentGatewayV2Address(destStateMachineId),
				to: this.ctx.source.configService.getIntentGatewayV2Address(souceStateMachineId),
			}

			protocolFeeInNativeToken = await this.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() =>
				this.ctx.dest.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() => 0n),
			)

			protocolFeeInNativeToken = (protocolFeeInNativeToken * 1005n) / 1000n
			postRequestFeeInDestFeeToken = (postRequestFeeInDestFeeToken * 1005n) / 1000n
		}

		const fillOptions: FillOptionsV2 = {
			relayerFee: postRequestFeeInDestFeeToken,
			nativeDispatchFee: protocolFeeInNativeToken,
			outputs: order.output.assets,
		}

		const totalNativeValue = totalEthValue + fillOptions.nativeDispatchFee

		const gasPrice = await this.ctx.dest.client.getGasPrice()
		const priorityFeeBumpPercent = params.maxPriorityFeePerGasBumpPercent ?? 8
		const maxFeeBumpPercent = params.maxFeePerGasBumpPercent ?? 10
		let maxPriorityFeePerGas = gasPrice + (gasPrice * BigInt(priorityFeeBumpPercent)) / 100n
		let maxFeePerGas = gasPrice + (gasPrice * BigInt(maxFeeBumpPercent)) / 100n

		const orderForEstimation = { ...order, session: solverAccountAddress }
		const commitment = orderV2Commitment(orderForEstimation)

		const fillOrderCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [transformOrderForContract(orderForEstimation), fillOptions],
		}) as HexString

		let callGasLimit: bigint = 500_000n
		let verificationGasLimit: bigint = 100_000n
		let preVerificationGas: bigint = 100_000n

		if (this.ctx.bundlerUrl) {
			try {
				const callData = this.crypto.encodeERC7821Execute([
					{ target: intentGatewayV2Address, value: totalNativeValue, data: fillOrderCalldata },
				])

				const accountGasLimits = this.crypto.packGasLimits(100_000n, callGasLimit)
				const gasFees = this.crypto.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

				const nonce = 0n

				const preliminaryUserOp: PackedUserOperation = {
					sender: solverAccountAddress,
					nonce,
					initCode: "0x" as HexString,
					callData: callData,
					accountGasLimits,
					preVerificationGas: 100_000n,
					gasFees,
					paymasterAndData: "0x" as HexString,
					signature: "0x" as HexString,
				}

				const userOpHash = this.crypto.computeUserOpHash(preliminaryUserOp, entryPointAddress, chainId)
				const messageHash = keccak256(
					concat([userOpHash, commitment as HexString, solverAccountAddress as import("viem").Hex]),
				)
				const solverSignature = await privateKeyToAccount(solverPrivateKey).signMessage({
					message: { raw: messageHash },
				})
				const solverSig = concat([commitment as HexString, solverSignature as import("viem").Hex]) as HexString

				const domainSeparator = this.crypto.getDomainSeparator(
					"IntentGateway",
					"2",
					chainId,
					intentGatewayV2Address,
				)
				const sessionSignature = await this.crypto.signSolverSelection(
					commitment as HexString,
					solverAccountAddress,
					domainSeparator,
					solverPrivateKey,
				)

				preliminaryUserOp.signature = concat([
					solverSig as import("viem").Hex,
					sessionSignature as import("viem").Hex,
				]) as HexString

				const bundlerUserOp = this.crypto.prepareBundlerCall(preliminaryUserOp)
				const gasEstimate = await this.crypto.sendBundler<BundlerGasEstimate>(
					BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS,
					[bundlerUserOp, entryPointAddress, bundlerStateOverrides],
				)

				callGasLimit = (BigInt(gasEstimate.callGasLimit) * 160n) / 100n
				verificationGasLimit = (BigInt(gasEstimate.verificationGasLimit) * 105n) / 100n
				preVerificationGas = (BigInt(gasEstimate.preVerificationGas) * 105n) / 100n

				// If using a Pimlico bundler, refine gas price using pimlico_getUserOperationGasPrice
				if (this.ctx.bundlerUrl?.toLowerCase().includes("pimlico.io")) {
					try {
						const pimlicoGasPrices = await this.crypto.sendBundler<PimlicoGasPriceEstimate>(
							BundlerMethod.PIMLICO_GET_USER_OPERATION_GAS_PRICE,
							[],
						)

						// Prefer fast quotes, then standard, then slow
						const level =
							pimlicoGasPrices.fast ?? pimlicoGasPrices.standard ?? pimlicoGasPrices.slow ?? null

						if (level) {
							const pimMaxFeePerGas = BigInt(level.maxFeePerGas)
							const pimMaxPriorityFeePerGas = BigInt(level.maxPriorityFeePerGas)

							maxFeePerGas = pimMaxFeePerGas + (pimMaxFeePerGas * BigInt(maxFeeBumpPercent)) / 100n
							maxPriorityFeePerGas =
								pimMaxPriorityFeePerGas +
								(pimMaxPriorityFeePerGas * BigInt(priorityFeeBumpPercent)) / 100n
						}
					} catch (e) {
						console.warn("Pimlico gas price fetch failed, using default gas price:", e)
					}
				}
			} catch (e) {
				console.warn("Bundler gas estimation failed, using fallback values:", e)
			}
		} else {
			try {
				const estimatedGas = await this.ctx.dest.client.estimateContractGas({
					abi: IntentGatewayV2ABI,
					address: intentGatewayV2Address,
					functionName: "fillOrder",
					args: [transformOrderForContract(order), fillOptions],
					account: solverAccountAddress,
					value: totalNativeValue,
					stateOverride: stateOverrides as any,
				})
				callGasLimit = (estimatedGas * 105n) / 100n
			} catch (e) {
				console.warn("fillOrder gas estimation failed, using fallback:", e)
			}
		}

		const totalGas = callGasLimit + verificationGasLimit + preVerificationGas
		const totalGasCostWei = totalGas * maxFeePerGas
		const totalGasInDestFeeToken = await convertGasToFeeToken(
			this.ctx,
			totalGas,
			"dest",
			destStateMachineId,
			gasPrice,
		)
		const totalGasInSourceFeeToken = adjustDecimals(
			totalGasInDestFeeToken,
			destFeeToken.decimals,
			sourceFeeToken.decimals,
		)

		return {
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
			totalGasCostWei,
			totalGasInFeeToken: totalGasInSourceFeeToken,
			fillOptions,
		}
	}

	/**
	 * Builds EVM state override objects for gas estimation of the `fillOrder`
	 * call, granting the solver account sufficient balances, allowances, and
	 * EntryPoint deposits so the estimation does not revert due to missing funds.
	 *
	 * Returns two formats of the same overrides:
	 * - `viem`: array format compatible with viem's `stateOverride` parameter.
	 * - `bundler`: object format compatible with the ERC-4337 bundler's
	 *   `eth_estimateUserOperationGas` state-override parameter.
	 *
	 * Optionally injects known solver account bytecode (from `solverCodeCache`)
	 * so the mock EOA used for estimation behaves like a real solver smart account.
	 *
	 * @param params.accountAddress - Address of the mock solver account.
	 * @param params.chain - State-machine ID of the destination chain.
	 * @param params.outputAssets - Token/amount pairs whose balance and allowance
	 *   slots should be overridden.
	 * @param params.spenderAddress - Address that needs allowance from the solver
	 *   account (i.e. the IntentGatewayV2 contract).
	 * @param params.intentGatewayV2Address - If provided, overrides slot 5 of
	 *   IntentGatewayV2 with the call-dispatcher address so dispatch calls
	 *   succeed during estimation.
	 * @param params.entryPointAddress - If provided, overrides the EntryPoint
	 *   deposit mapping to give the solver account a large deposit.
	 * @returns An object with `viem` and `bundler` state-override collections.
	 */
	async buildStateOverride(params: {
		accountAddress: HexString
		chain: string
		outputAssets: { token: HexString; amount: bigint }[]
		spenderAddress: HexString
		intentGatewayV2Address?: HexString
		entryPointAddress?: HexString
	}): Promise<{
		viem: { address: HexString; balance?: bigint; stateDiff?: { slot: HexString; value: HexString }[] }[]
		bundler: Record<string, { balance?: string; stateDiff?: Record<string, string>; code?: string }>
	}> {
		const { accountAddress, chain, outputAssets, spenderAddress, intentGatewayV2Address, entryPointAddress } =
			params
		const testValue = toHex(maxUint256 / 2n, { size: 32 }) as HexString

		const viemOverrides: {
			address: HexString
			balance?: bigint
			stateDiff?: { slot: HexString; value: HexString }[]
		}[] = []
		const bundlerOverrides: Record<
			string,
			{ balance?: string; stateDiff?: Record<string, string>; code?: string }
		> = {}

		if (intentGatewayV2Address) {
			const paramsSlot5 = pad(toHex(5n), { size: 32 }) as HexString
			const dispatcherAddress = this.ctx.dest.configService.getCalldispatcherAddress(chain)
			const newSlot5Value = ("0x" + "0".repeat(22) + "00" + dispatcherAddress.slice(2).toLowerCase()) as HexString

			viemOverrides.push({
				address: intentGatewayV2Address,
				stateDiff: [{ slot: paramsSlot5, value: newSlot5Value }],
			})
			bundlerOverrides[intentGatewayV2Address] = {
				stateDiff: { [paramsSlot5]: newSlot5Value },
			}
		}

		if (entryPointAddress) {
			const entryPointDepositSlot = calculateBalanceMappingLocation(0n, accountAddress, EvmLanguage.Solidity)

			viemOverrides.push({
				address: entryPointAddress,
				stateDiff: [{ slot: entryPointDepositSlot, value: testValue }],
			})
			bundlerOverrides[entryPointAddress] = {
				stateDiff: { [entryPointDepositSlot]: testValue },
			}
		}

		viemOverrides.push({
			address: accountAddress,
			balance: maxUint256,
		})
		bundlerOverrides[accountAddress] = {
			balance: testValue,
		}

		for (const output of outputAssets) {
			const tokenAddress = bytes32ToBytes20(output.token)

			if (tokenAddress === ADDRESS_ZERO) {
				continue
			}

			try {
				const viemStateDiffs: { slot: HexString; value: HexString }[] = []
				const bundlerStateDiffs: Record<string, string> = {}

				const balanceData = (ERC20Method.BALANCE_OF + bytes20ToBytes32(accountAddress).slice(2)) as HexString
				const balanceSlot = await getOrFetchStorageSlot(this.ctx.dest.client, chain, tokenAddress, balanceData)
				if (balanceSlot) {
					viemStateDiffs.push({ slot: balanceSlot, value: testValue })
					bundlerStateDiffs[balanceSlot] = testValue
				}

				try {
					const allowanceData = (ERC20Method.ALLOWANCE +
						bytes20ToBytes32(accountAddress).slice(2) +
						bytes20ToBytes32(spenderAddress).slice(2)) as HexString
					const allowanceSlot = await getOrFetchStorageSlot(
						this.ctx.dest.client,
						chain,
						tokenAddress,
						allowanceData,
					)
					if (allowanceSlot) {
						viemStateDiffs.push({ slot: allowanceSlot, value: testValue })
						bundlerStateDiffs[allowanceSlot] = testValue
					}
				} catch {
					// Allowance slot not found
				}

				if (viemStateDiffs.length > 0) {
					viemOverrides.push({ address: tokenAddress, stateDiff: viemStateDiffs })
				}
				if (Object.keys(bundlerStateDiffs).length > 0) {
					bundlerOverrides[tokenAddress] = { stateDiff: bundlerStateDiffs }
				}
			} catch {
				// Balance slot not found
			}
		}

		const solverAccountContract = this.ctx.dest.configService.getSolverAccountAddress(chain)
		if (solverAccountContract) {
			try {
				const cacheKey = solverAccountContract.toLowerCase()
				let solverCode = this.ctx.solverCodeCache.get(cacheKey)

				if (!solverCode) {
					solverCode = await this.ctx.dest.client.getCode({ address: solverAccountContract })
					if (solverCode && solverCode !== "0x") {
						this.ctx.solverCodeCache.set(cacheKey, solverCode)
					}
				}

				if (solverCode && solverCode !== "0x") {
					if (!bundlerOverrides[accountAddress]) {
						bundlerOverrides[accountAddress] = {}
					}
					bundlerOverrides[accountAddress].code = solverCode
				}
			} catch {
				// Ignore
			}
		}

		return { viem: viemOverrides, bundler: bundlerOverrides }
	}

	/**
	 * Quotes the native token cost of dispatching an ISMP POST request through
	 * the IntentGateway (v1) `quoteNative` function.
	 *
	 * Uses the v1 IntentGateway ABI (not IntentGatewayV2) because the dispatch
	 * call is routed through the legacy gateway contract.
	 *
	 * @param postRequest - The ISMP POST request to quote.
	 * @param fee - The relayer fee (in dest fee token) to include in the quote.
	 * @returns The native token amount required to dispatch the request.
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

		return await this.ctx.dest.client.readContract({
			address: this.ctx.dest.configService.getIntentGatewayAddress(postRequest.dest),
			abi: IntentGateway.ABI,
			functionName: "quoteNative",
			args: [dispatchPost],
		})
	}
}
