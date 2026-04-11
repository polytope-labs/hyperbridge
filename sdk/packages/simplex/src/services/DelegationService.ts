import type { HexString } from "@hyperbridge/sdk"
import { CryptoUtils, BundlerMethod } from "@hyperbridge/sdk"
import { concat, keccak256, toHex, toRlp, zeroAddress } from "viem"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import type { SigningAccount } from "./wallet"
import { buildPaymasterAndData, hasPaymaster } from "./paymaster"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/** Floor for set-code (0x04) txs; L2s like Arbitrum reject viem's default ~94k with "intrinsic gas too low". */
const DELEGATION_TX_GAS_FLOOR = 350_000n

/**
 * Service for managing EIP-7702 delegation of the filler's EOA to the SolverAccount contract.
 * This enables the filler to participate in solver selection mode.
 *
 * When a SimplexPaymaster is configured, delegation is performed via a no-op UserOp
 * sent through the bundler — the paymaster pays gas in ERC-20 tokens, so the solver
 * never needs native tokens. Falls back to a direct type-0x04 tx if the bundler
 * path is unavailable.
 */
export class DelegationService {
	private logger = getLogger("delegation-service")

	constructor(
		private clientManager: ChainClientManager,
		private configService: FillerConfigService,
		private signer: SigningAccount,
	) {}

	private computeAuthorizationHash(chainId: number, contractAddress: HexString, nonce: number): HexString {
		const encoded = toRlp([toHex(chainId), contractAddress, toHex(nonce)])
		return keccak256(concat(["0x05", encoded])) as HexString
	}

	/**
	 * @param viaBundler When true, uses the current nonce (bundler submits the tx).
	 *                   When false, uses nonce+1 (EOA submits the type-0x04 tx itself).
	 */
	private async buildAuthorization(
		chain: string,
		contractAddress: HexString,
		viaBundler = false,
	): Promise<{
		chainId: number
		address: HexString
		nonce: number
		r: HexString
		s: HexString
		yParity: number
	}> {
		const publicClient = this.clientManager.getPublicClient(chain)
		const chainId = this.configService.getChainId(chain)
		const authorityAddress = this.signer.account.address as HexString
		const currentNonce = await publicClient.getTransactionCount({
			address: authorityAddress,
			blockTag: "latest",
		})
		const authorizationNonce = viaBundler ? currentNonce : currentNonce + 1
		const authHash = this.computeAuthorizationHash(chainId, contractAddress, Number(authorizationNonce))
		const { r, s, yParity } = await this.signer.signRawHash(authHash)

		return {
			chainId,
			address: contractAddress,
			nonce: authorizationNonce,
			r,
			s,
			yParity,
		}
	}

	private async sendDelegationTransaction(
		chain: string,
		authorization: {
			chainId: number
			address: HexString
			nonce: number
			r: HexString
			s: HexString
			yParity: number
		},
	): Promise<HexString> {
		const authorityAddress = this.signer.account.address
		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		return this.signer.sendEip7702DelegationTransaction({
			walletClient,
			publicClient,
			authorityAddress,
			authorization,
			chainIdFallback: this.configService.getChainId(chain),
			gasFloor: DELEGATION_TX_GAS_FLOOR,
		})
	}

	/**
	 * Checks if the filler's EOA is already delegated to the SolverAccount contract on a specific chain.
	 */
	async isDelegated(chain: string): Promise<boolean> {
		const client = this.clientManager.getPublicClient(chain)
		const account = this.signer.account
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			return false
		}

		try {
			const code = await client.getCode({ address: account.address })

			if (!code || code === "0x") {
				return false
			}

			if (code.toLowerCase().startsWith(DELEGATION_INDICATOR_PREFIX)) {
				const delegatedTo = ("0x" + code.slice(8)) as HexString
				const isCorrectDelegate = delegatedTo.toLowerCase() === solverAccountContract.toLowerCase()

				this.logger.debug(
					{ chain, delegatedTo, expected: solverAccountContract, isCorrectDelegate },
					"Checked delegation status",
				)

				return isCorrectDelegate
			}

			return false
		} catch (error) {
			this.logger.error({ chain, error }, "Failed to check delegation status")
			return false
		}
	}

	/**
	 * Sets up EIP-7702 delegation via the bundler with a no-op UserOp.
	 * Prefers Circle Paymaster (USDC permit) when available and filler has USDC balance.
	 * Falls back to SimplexPaymaster with forceApproveMode otherwise.
	 */
	private async setupDelegationViaBundler(chain: string): Promise<boolean> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)
		const entryPointAddress = this.configService.getEntryPointAddress(chain)
		const bundlerUrl = this.configService.getBundlerUrl(chain)

		if (!solverAccountContract || !hasPaymaster(chain, this.configService) || !entryPointAddress || !bundlerUrl) {
			this.logger.warn({ chain }, "Missing config for bundler-based delegation, falling back to direct tx")
			return false
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const walletClient = this.clientManager.getWalletClient(chain)
		const solverAccount = this.signer.account.address as HexString
		const chainId = this.configService.getChainId(chain)

		try {
			// Build EIP-7702 authorization (bundler submits tx, so use current nonce)
			const authorization = await this.buildAuthorization(chain, solverAccountContract, true)

			this.logger.info(
				{ chain, solverAccount, solverAccountContract, mode: "bundler" },
				"Setting up EIP-7702 delegation via bundler with paymaster",
			)

			// Build paymaster data — Circle (USDC permit) → Simplex (approve) → none
			const pmResult = await buildPaymasterAndData({
				chain,
				solverAccount,
				publicClient,
				walletClient,
				signer: this.signer,
				configService: this.configService,
				forceApproveMode: true,
			})
			if (pmResult.type === "none") {
				this.logger.warn({ chain }, "No paymaster available for delegation")
				return false
			}
			const paymasterAndData = pmResult.paymasterAndData
			this.logger.info(
				{ chain, paymaster: pmResult.address, type: pmResult.type },
				"Using paymaster for delegation UserOp",
			)

			// Get gas prices — detect bundler type and use appropriate RPC method
			let maxFeePerGas: bigint
			let maxPriorityFeePerGas: bigint

			const bundlerUrlLower = bundlerUrl.toLowerCase()
			const isPimlico = bundlerUrlLower.includes("pimlico.io")
			const isAlchemy = bundlerUrlLower.includes("alchemy.com")

			if (isPimlico) {
				const gasPriceResult = await this.sendBundlerRpc<{
					fast: { maxFeePerGas: string; maxPriorityFeePerGas: string }
				}>(bundlerUrl, BundlerMethod.PIMLICO_GET_USER_OPERATION_GAS_PRICE, [])
				maxFeePerGas = BigInt(gasPriceResult.fast.maxFeePerGas)
				maxPriorityFeePerGas = BigInt(gasPriceResult.fast.maxPriorityFeePerGas)
			} else if (isAlchemy) {
				const [rundlerPriorityFee, latestBlock] = await Promise.all([
					this.sendBundlerRpc<HexString>(bundlerUrl, BundlerMethod.RUNDLER_MAX_PRIORITY_FEE_PER_GAS, []),
					publicClient.getBlock({ blockTag: "latest" }),
				])
				const baseFeePerGas = latestBlock.baseFeePerGas ?? (await publicClient.getGasPrice())
				const chainIdBigInt = BigInt(chainId)
				const isArbitrum = chainIdBigInt === 42161n
				const alchemyPrioBump = isArbitrum ? 0n : 25n
				maxPriorityFeePerGas =
					BigInt(rundlerPriorityFee) + (BigInt(rundlerPriorityFee) * alchemyPrioBump) / 100n
				const bufferedBaseFee = baseFeePerGas + (baseFeePerGas * 50n) / 100n
				maxFeePerGas = bufferedBaseFee + maxPriorityFeePerGas
			} else {
				const gasPrice = await publicClient.getGasPrice()
				maxPriorityFeePerGas = gasPrice + (gasPrice * 8n) / 100n
				maxFeePerGas = gasPrice + (gasPrice * 10n) / 100n
			}

			// Get nonce from EntryPoint (key = 0 for delegation UserOps)
			const nonce = (await publicClient.readContract({
				address: entryPointAddress,
				abi: ENTRYPOINT_ABI,
				functionName: "getNonce",
				args: [solverAccount, 0n],
			})) as bigint

			// 5. Build minimal no-op UserOp
			const verificationGasLimit = 150_000n
			const callGasLimit = 50_000n
			const preVerificationGas = 100_000n
			const accountGasLimits = CryptoUtils.packGasLimits(verificationGasLimit, callGasLimit)
			const gasFees = CryptoUtils.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

			const userOp = {
				sender: solverAccount,
				nonce,
				initCode: "0x" as HexString,
				callData: "0x" as HexString,
				accountGasLimits,
				preVerificationGas,
				gasFees,
				paymasterAndData,
				signature: "0x" as HexString,
			}

			// Compute UserOp hash (EIP-712) and sign with raw ECDSA (no eth prefix)
			//    SolverAccount._rawSignatureValidation expects ECDSA.recover(userOpHash, sig) == address(this)
			const userOpHash = CryptoUtils.computeUserOpHash(
				userOp,
				entryPointAddress as `0x${string}`,
				BigInt(chainId),
			)
			const { r, s, yParity } = await this.signer.signRawHash(userOpHash as HexString)
			const v = yParity === 0 ? 27 : 28
			userOp.signature = concat([r, s, toHex(v)]) as HexString

			// Prepare bundler call format using CryptoUtils
			const bundlerUserOp = CryptoUtils.prepareBundlerCall(userOp)

			// Attach EIP-7702 authorization inside the UserOp object for Pimlico
			bundlerUserOp.eip7702Auth = {
				address: authorization.address,
				chainId: toHex(authorization.chainId),
				nonce: toHex(authorization.nonce),
				r: authorization.r,
				s: authorization.s,
				yParity: toHex(authorization.yParity),
			}

			// Send to bundler: eth_sendUserOperation(userOp, entryPoint)
			const userOpHashResult = await this.sendBundlerRpc<HexString>(
				bundlerUrl,
				BundlerMethod.ETH_SEND_USER_OPERATION,
				[bundlerUserOp, entryPointAddress],
			)

			this.logger.info({ chain, userOpHash: userOpHashResult }, "Delegation UserOp submitted to bundler")

			// Wait for receipt
			const receipt = await this.waitForUserOpReceipt(bundlerUrl, userOpHashResult)

			if (receipt) {
				this.logger.info(
					{ chain, txHash: receipt.receipt?.transactionHash },
					"Delegation via bundler successful — paymaster paid gas",
				)
				return true
			}

			this.logger.warn({ chain }, "Delegation UserOp receipt not received, checking on-chain status")
			return this.isDelegated(chain)
		} catch (error) {
			this.logger.warn({ chain, error }, "Bundler delegation failed, will fall back to direct tx")
			return false
		}
	}

	/**
	 * Sets up EIP-7702 delegation from the filler's EOA to the SolverAccount contract.
	 *
	 * Tries bundler path first (paymaster pays gas in ERC-20).
	 * Falls back to direct type-0x04 tx if bundler path fails.
	 */
	async setupDelegation(chain: string): Promise<boolean> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			this.logger.error("solverAccountContractAddress not configured")
			return false
		}

		if (await this.isDelegated(chain)) {
			this.logger.info({ chain }, "EOA already delegated to SolverAccount")
			return true
		}

		// Try bundler path first (paymaster pays gas)
		if (hasPaymaster(chain, this.configService)) {
			const success = await this.setupDelegationViaBundler(chain)
			if (success) return true
			this.logger.info({ chain }, "Falling back to direct delegation tx")
		}

		// Fallback: direct type-0x04 transaction (requires native token)
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info(
				{ chain, authority: this.signer.account.address, solverAccountContract, mode: this.signer.mode },
				"Setting up EIP-7702 delegation via direct tx",
			)

			const authorization = await this.buildAuthorization(chain, solverAccountContract)
			const hash = await this.sendDelegationTransaction(chain, authorization)

			this.logger.info({ chain, txHash: hash }, "Delegation transaction sent")

			const receipt = await publicClient.waitForTransactionReceipt({ hash })

			if (receipt.status === "success") {
				this.logger.info({ chain, txHash: hash, blockNumber: receipt.blockNumber }, "Delegation successful")
				return true
			}

			this.logger.error({ chain, txHash: hash, status: receipt.status }, "Delegation transaction failed")
			return false
		} catch (error) {
			this.logger.error({ chain, error }, "Failed to setup delegation")
			return false
		}
	}

	/**
	 * Sets up delegation on the specified chains where solver selection is active.
	 */
	async setupDelegationOnChains(chains: string[]): Promise<{ success: boolean; results: Record<string, boolean> }> {
		const results: Record<string, boolean> = {}
		let allSuccess = true

		for (const chain of chains) {
			try {
				results[chain] = await this.setupDelegation(chain)
				if (!results[chain]) {
					allSuccess = false
				}
			} catch (error) {
				this.logger.error({ chain, error }, "Failed to setup delegation on chain")
				results[chain] = false
				allSuccess = false
			}
		}

		return { success: allSuccess, results }
	}

	/**
	 * Revokes delegation by delegating to the zero address.
	 */
	async revokeDelegation(chain: string): Promise<boolean> {
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info(
				{ chain, authority: this.signer.account.address, mode: this.signer.mode },
				"Revoking EIP-7702 delegation",
			)

			const authorization = await this.buildAuthorization(chain, zeroAddress)
			const hash = await this.sendDelegationTransaction(chain, authorization)

			const receipt = await publicClient.waitForTransactionReceipt({ hash })

			if (receipt.status === "success") {
				this.logger.info({ chain, txHash: hash }, "Delegation revoked successfully")
				return true
			}

			return false
		} catch (error) {
			this.logger.error({ chain, error }, "Failed to revoke delegation")
			return false
		}
	}

	// ── Helpers ──────────────────────────────────────────────────────

	private async sendBundlerRpc<T>(bundlerUrl: string, method: string, params: unknown[]): Promise<T> {
		const response = await fetch(bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
		})

		const result = (await response.json()) as { result?: T; error?: { message?: string } }

		if (result.error) {
			throw new Error(`Bundler RPC error (${method}): ${result.error.message || JSON.stringify(result.error)}`)
		}

		return result.result as T
	}

	private async waitForUserOpReceipt(
		bundlerUrl: string,
		userOpHash: HexString,
		maxAttempts = 30,
		intervalMs = 2000,
	): Promise<{ receipt?: { transactionHash: HexString } } | null> {
		for (let i = 0; i < maxAttempts; i++) {
			try {
				const receipt = await this.sendBundlerRpc<{ receipt: { transactionHash: HexString } } | null>(
					bundlerUrl,
					BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT,
					[userOpHash],
				)
				if (receipt) return receipt
			} catch {
				// Not found yet
			}
			await new Promise((resolve) => setTimeout(resolve, intervalMs))
		}
		return null
	}
}
