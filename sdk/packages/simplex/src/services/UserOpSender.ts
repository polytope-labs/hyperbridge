import { CryptoUtils, BundlerMethod, type PackedUserOperation, type HexString } from "@hyperbridge/sdk"
import { concat, toHex, type PublicClient } from "viem"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import type { SigningAccount } from "./wallet"
import { buildPaymasterAndData, getUsdcBalanceStatus, hasPaymaster } from "./paymaster"

/**
 * Raw EIP-7702 authorization, as produced by the delegation flow. Attached to the
 * bundler UserOp so a not-yet-delegated EOA can be delegated in the same op.
 */
export interface Eip7702Authorization {
	chainId: number
	address: HexString
	nonce: number
	r: HexString
	s: HexString
	yParity: number
}

export interface UserOpGasLimits {
	verificationGasLimit: bigint
	callGasLimit: bigint
	preVerificationGas: bigint
}

export interface SponsoredUserOpRequest {
	chain: string
	/** ERC-7821 batch (or "0x" for a no-op, e.g. delegation). */
	callData: HexString
	/** Attach when the EOA still needs delegating in this op. */
	eip7702Auth?: Eip7702Authorization
	/** EntryPoint nonce key. Defaults to 0. */
	nonceKey?: bigint
	/**
	 * Explicit gas limits. Provide them to skip bundler estimation — required for
	 * the not-yet-delegated EIP-7702 case, where the account has no code to
	 * simulate so estimation is unreliable. When omitted, limits are estimated.
	 */
	gas?: UserOpGasLimits
	/**
	 * Override for the Circle paymaster verification gas limit (default 200k).
	 * Lower it for a known cheap op so rundler's verification-gas-limit efficiency
	 * policy — which divides actual usage by `accountVerif + paymasterVerif` —
	 * accepts the op (e.g. re-delegation).
	 */
	paymasterVerificationGasLimit?: bigint
}

// Generous fallbacks used only when bundler gas estimation fails. The paymaster
// refunds unused gas in postOp, and the permit ceiling caps the validation-phase
// prefund regardless of these, so over-estimating here is safe (it does not pull
// more USDC than the allowance).
const FALLBACK_VERIFICATION_GAS_LIMIT = 250_000n
const FALLBACK_CALL_GAS_LIMIT = 1_500_000n
const FALLBACK_PRE_VERIFICATION_GAS = 150_000n

/**
 * Submits self-initiated, Circle-Paymaster-sponsored UserOperations through the
 * bundler, so the solver pays gas in USDC rather than native token. Shared by the
 * delegation setup and the vault sweep/redeem paths.
 *
 * Submission contract (so callers can safely fall back to a native tx without
 * risking a double-execution):
 * - Returns `null` when the op was **never submitted** — paymaster/bundler not
 *   configured, insufficient USDC, or the bundler rejected `eth_sendUserOperation`
 *   outright (it never entered the mempool). The caller may fall back to native.
 * - Returns `{ txHash }` once a receipt is observed.
 * - **Throws** only when the op **was submitted** but no receipt arrived. The
 *   caller must NOT fall back (the op may still land) — retry on the next cycle.
 */
export class UserOpSender {
	private logger = getLogger("userop-sender")

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly configService: FillerConfigService,
		private readonly signer: SigningAccount,
	) {}

	/** True when a sponsored UserOp is even possible on this chain (paymaster + bundler configured). */
	canSponsor(chain: string): boolean {
		return (
			hasPaymaster(chain, this.configService) &&
			!!this.configService.getEntryPointAddress(chain) &&
			!!this.configService.getBundlerUrl(chain)
		)
	}

	async trySendSponsored(req: SponsoredUserOpRequest): Promise<{ txHash: HexString } | null> {
		const { chain, callData, eip7702Auth, nonceKey = 0n, gas, paymasterVerificationGasLimit } = req

		const entryPoint = this.configService.getEntryPointAddress(chain)
		const bundlerUrl = this.configService.getBundlerUrl(chain)
		if (!hasPaymaster(chain, this.configService) || !entryPoint || !bundlerUrl) {
			return null
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const walletClient = this.clientManager.getWalletClient(chain)
		const solverAccount = this.signer.account.address as HexString
		const chainId = this.configService.getChainId(chain)

		// The paymaster sponsors gas in USDC; without enough USDC it can't pay.
		const usdcDecimals = this.configService.getUsdcDecimals(chain)
		const usdc = await getUsdcBalanceStatus(
			publicClient,
			solverAccount,
			this.configService.getUsdcAsset(chain),
			usdcDecimals,
		)
		if (!usdc.sufficient) {
			this.logger.warn(
				{ chain, solverAccount, usdcBalance: usdc.balance.toString(), required: usdc.required.toString() },
				"Insufficient USDC to sponsor UserOp via paymaster; caller should fall back to native",
			)
			return null
		}

		const pm = await buildPaymasterAndData({
			chain,
			solverAccount,
			publicClient,
			walletClient,
			signer: this.signer,
			configService: this.configService,
			paymasterVerificationGasLimit,
		})
		if (pm.type === "none") {
			this.logger.warn({ chain }, "Paymaster data unavailable; caller should fall back to native")
			return null
		}
		const paymasterAndData = pm.paymasterAndData

		const { maxFeePerGas, maxPriorityFeePerGas } = await this.getGasPrice(bundlerUrl, publicClient, chainId)

		const nonce = (await publicClient.readContract({
			address: entryPoint,
			abi: ENTRYPOINT_ABI,
			functionName: "getNonce",
			args: [solverAccount, nonceKey],
		})) as bigint

		const formattedAuth = eip7702Auth
			? {
					address: eip7702Auth.address,
					chainId: toHex(eip7702Auth.chainId),
					nonce: toHex(eip7702Auth.nonce),
					r: eip7702Auth.r,
					s: eip7702Auth.s,
					yParity: toHex(eip7702Auth.yParity),
				}
			: undefined

		// Explicit limits skip estimation — the bundler echoes the input limits for
		// these ops rather than simulating, so callers pass measured fixed limits.
		const { verificationGasLimit, callGasLimit, preVerificationGas } =
			gas ??
			(await this.estimateGas({
				bundlerUrl,
				entryPoint,
				chainId,
				solverAccount,
				nonce,
				callData,
				paymasterAndData,
				maxFeePerGas,
				maxPriorityFeePerGas,
				formattedAuth,
			}))

		const userOp = await this.buildSignedUserOp({
			entryPoint,
			chainId,
			solverAccount,
			nonce,
			callData,
			paymasterAndData,
			maxFeePerGas,
			maxPriorityFeePerGas,
			verificationGasLimit,
			callGasLimit,
			preVerificationGas,
		})

		const bundlerUserOp = CryptoUtils.prepareBundlerCall(userOp)
		if (formattedAuth) bundlerUserOp.eip7702Auth = formattedAuth

		let userOpHash: HexString
		try {
			userOpHash = await this.sendBundlerRpc<HexString>(bundlerUrl, BundlerMethod.ETH_SEND_USER_OPERATION, [
				bundlerUserOp,
				entryPoint,
			])
		} catch (error) {
			// Rejected before entering the mempool — never submitted, safe to fall back.
			this.logger.warn({ chain, error }, "Bundler rejected UserOp; caller should fall back to native")
			return null
		}

		this.logger.info({ chain, userOpHash }, "Sponsored UserOp submitted to bundler")

		const receipt = await this.waitForUserOpReceipt(bundlerUrl, userOpHash)
		const txHash = receipt?.receipt?.transactionHash
		if (!txHash) {
			// Submitted but unconfirmed — it may still land, so the caller must NOT
			// fall back to a native tx (that would double-execute).
			throw new Error(`Sponsored UserOp ${userOpHash} submitted but no receipt within timeout`)
		}
		if (receipt?.success === false) {
			// Included but the inner execution reverted — surface it rather than report
			// success. The caller must not fall back (the op was mined).
			throw new Error(`Sponsored UserOp ${userOpHash} reverted on-chain (tx ${txHash})`)
		}

		this.logger.info({ chain, userOpHash, txHash }, "Sponsored UserOp confirmed — paymaster paid gas")
		return { txHash }
	}

	// ── Internals ────────────────────────────────────────────────────────

	/**
	 * Estimates account gas limits via the bundler. The op is signed first so the
	 * estimation simulation passes the account's ECDSA validation. Falls back to
	 * generous constants when the bundler estimate is unavailable.
	 */
	private async estimateGas(p: {
		bundlerUrl: string
		entryPoint: HexString
		chainId: number
		solverAccount: HexString
		nonce: bigint
		callData: HexString
		paymasterAndData: HexString
		maxFeePerGas: bigint
		maxPriorityFeePerGas: bigint
		formattedAuth?: Record<string, unknown>
	}): Promise<{ verificationGasLimit: bigint; callGasLimit: bigint; preVerificationGas: bigint }> {
		try {
			const prelim = await this.buildSignedUserOp({
				entryPoint: p.entryPoint,
				chainId: p.chainId,
				solverAccount: p.solverAccount,
				nonce: p.nonce,
				callData: p.callData,
				paymasterAndData: p.paymasterAndData,
				maxFeePerGas: p.maxFeePerGas,
				maxPriorityFeePerGas: p.maxPriorityFeePerGas,
				verificationGasLimit: FALLBACK_VERIFICATION_GAS_LIMIT,
				callGasLimit: FALLBACK_CALL_GAS_LIMIT,
				preVerificationGas: FALLBACK_PRE_VERIFICATION_GAS,
			})

			const bundlerUserOp = CryptoUtils.prepareBundlerCall(prelim)
			if (p.formattedAuth) bundlerUserOp.eip7702Auth = p.formattedAuth

			const est = await this.sendBundlerRpc<{
				callGasLimit: string
				verificationGasLimit: string
				preVerificationGas: string
			}>(p.bundlerUrl, BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS, [bundlerUserOp, p.entryPoint])

			return {
				// Bias the execution limit up — sweep/redeem batches vary in size.
				callGasLimit: (BigInt(est.callGasLimit) * 160n) / 100n,
				verificationGasLimit: (BigInt(est.verificationGasLimit) * 110n) / 100n,
				preVerificationGas: (BigInt(est.preVerificationGas) * 110n) / 100n,
			}
		} catch (error) {
			this.logger.warn({ error }, "UserOp gas estimation failed, using generous fallback limits")
			return {
				verificationGasLimit: FALLBACK_VERIFICATION_GAS_LIMIT,
				callGasLimit: FALLBACK_CALL_GAS_LIMIT,
				preVerificationGas: FALLBACK_PRE_VERIFICATION_GAS,
			}
		}
	}

	private async buildSignedUserOp(p: {
		entryPoint: HexString
		chainId: number
		solverAccount: HexString
		nonce: bigint
		callData: HexString
		paymasterAndData: HexString
		maxFeePerGas: bigint
		maxPriorityFeePerGas: bigint
		verificationGasLimit: bigint
		callGasLimit: bigint
		preVerificationGas: bigint
	}): Promise<PackedUserOperation> {
		const userOp: PackedUserOperation = {
			sender: p.solverAccount,
			nonce: p.nonce,
			initCode: "0x" as HexString,
			callData: p.callData,
			accountGasLimits: CryptoUtils.packGasLimits(p.verificationGasLimit, p.callGasLimit),
			preVerificationGas: p.preVerificationGas,
			gasFees: CryptoUtils.packGasFees(p.maxPriorityFeePerGas, p.maxFeePerGas),
			paymasterAndData: p.paymasterAndData,
			signature: "0x" as HexString,
		}

		// SolverAccount._rawSignatureValidation expects ECDSA.recover(userOpHash, sig) == account.
		const userOpHash = CryptoUtils.computeUserOpHash(userOp, p.entryPoint, BigInt(p.chainId))
		const { r, s, yParity } = await this.signer.signRawHash(userOpHash as HexString)
		const v = yParity === 0 ? 27 : 28
		userOp.signature = concat([r, s, toHex(v)]) as HexString
		return userOp
	}

	/** Mirrors the gas-price selection used by the delegation and fill paths. */
	private async getGasPrice(
		bundlerUrl: string,
		publicClient: PublicClient,
		chainId: number,
	): Promise<{ maxFeePerGas: bigint; maxPriorityFeePerGas: bigint }> {
		const lower = bundlerUrl.toLowerCase()
		if (lower.includes("pimlico.io")) {
			const res = await this.sendBundlerRpc<{ fast: { maxFeePerGas: string; maxPriorityFeePerGas: string } }>(
				bundlerUrl,
				BundlerMethod.PIMLICO_GET_USER_OPERATION_GAS_PRICE,
				[],
			)
			return { maxFeePerGas: BigInt(res.fast.maxFeePerGas), maxPriorityFeePerGas: BigInt(res.fast.maxPriorityFeePerGas) }
		}
		if (lower.includes("alchemy.com")) {
			const [rundlerPriorityFee, latestBlock] = await Promise.all([
				this.sendBundlerRpc<HexString>(bundlerUrl, BundlerMethod.RUNDLER_MAX_PRIORITY_FEE_PER_GAS, []),
				publicClient.getBlock({ blockTag: "latest" }),
			])
			const baseFeePerGas = latestBlock.baseFeePerGas ?? (await publicClient.getGasPrice())
			const isArbitrum = BigInt(chainId) === 42161n
			const prioBump = isArbitrum ? 0n : 25n
			const maxPriorityFeePerGas = BigInt(rundlerPriorityFee) + (BigInt(rundlerPriorityFee) * prioBump) / 100n
			const bufferedBaseFee = baseFeePerGas + (baseFeePerGas * 50n) / 100n
			return { maxFeePerGas: bufferedBaseFee + maxPriorityFeePerGas, maxPriorityFeePerGas }
		}
		const gasPrice = await publicClient.getGasPrice()
		return { maxFeePerGas: gasPrice + (gasPrice * 10n) / 100n, maxPriorityFeePerGas: gasPrice + (gasPrice * 8n) / 100n }
	}

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
	): Promise<{ success?: boolean; receipt?: { transactionHash: HexString } } | null> {
		for (let i = 0; i < maxAttempts; i++) {
			try {
				const receipt = await this.sendBundlerRpc<{
					success?: boolean
					receipt: { transactionHash: HexString }
				} | null>(bundlerUrl, BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT, [userOpHash])
				if (receipt) return receipt
			} catch {
				// Not indexed yet.
			}
			await new Promise((resolve) => setTimeout(resolve, intervalMs))
		}
		return null
	}
}
