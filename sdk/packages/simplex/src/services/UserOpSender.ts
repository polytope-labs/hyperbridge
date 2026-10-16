import {
	CryptoUtils,
	BundlerMethod,
	rundlerUserOperationFees,
	type PackedUserOperation,
	type HexString,
} from "@hyperbridge/sdk"
import { hashTypedData, toHex, type PublicClient } from "viem"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { type Logger , moduleLogger} from "./Logger"
import type { Signer } from "./wallet"
import { buildPaymasterAndData, hasPaymaster } from "./paymaster"
import type { PaymasterDataResult } from "./paymaster"
import { SubmittedUnconfirmedError, type SubmittedUserOp } from "./submission"

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
	/**
	 * Attach when the EOA still needs delegating in this op. Pass a factory rather
	 * than a pre-signed tuple: building paymaster data can send an approve tx from
	 * the authority EOA (the Simplex bootstrap approve to Permit2), and an authorization signed before
	 * that tx embeds a stale nonce the bundler will reject. The factory runs only
	 * after paymaster data is built.
	 */
	eip7702Auth?: Eip7702Authorization | (() => Promise<Eip7702Authorization>)
	/** EntryPoint nonce key. Defaults to 0. */
	nonceKey?: bigint
	/**
	 * Explicit gas limits. Provide them to skip bundler estimation — required for
	 * the not-yet-delegated EIP-7702 case, where the account has no code to
	 * simulate so estimation is unreliable. When omitted, limits are estimated.
	 */
	gas?: UserOpGasLimits
	/**
	 * Lets the paymaster fall back to an EIP-2612 permit when the fee token has no
	 * Permit2 allowance yet. Only a first-time delegation sets this — it carries the
	 * `approve(Permit2, max)` in its own callData, so the permit buys the one op that
	 * makes every later Permit2 op possible without native.
	 */
	permitBootstrap?: boolean
}

// Generous fallbacks used only when bundler gas estimation fails. The paymaster
// refunds unused gas in postOp, and the signed Permit2 amount caps the validation-phase
// prefund regardless of these, so over-estimating here is safe (it does not pull
// more USDC than the solver authorized).
const FALLBACK_VERIFICATION_GAS_LIMIT = 250_000n
const FALLBACK_CALL_GAS_LIMIT = 1_500_000n
const FALLBACK_PRE_VERIFICATION_GAS = 150_000n

const PRIORITY_FEE_BUMP_PERCENT = 8n
const MAX_FEE_BUMP_PERCENT = 10n

// Bounds the cheap bundler reads — the receipt and by-hash polls. A gateway that accepts the
// connection but never answers would otherwise pin the send path indefinitely; the receipt wait
// is time-based, so each poll must finish well within it.
const BUNDLER_POLL_TIMEOUT_MS = 10_000

// The send and estimate calls run the whole validation simulation, so they get a longer bound: a
// slow-but-healthy send must not be aborted into the ambiguous path (which forces a lookup and
// may read as unconfirmed), nor an estimate into the ~1.9M-gas fallback limits.
const BUNDLER_SIMULATION_TIMEOUT_MS = 30_000

// A receipt is polled every `RECEIPT_POLL_INTERVAL_MS` until this wall-clock deadline. Time-based
// rather than a poll count so the per-poll timeout cannot stretch the wait (a 30-poll count with a
// 10 s poll timeout would run to six minutes).
const RECEIPT_WAIT_DEADLINE_MS = 90_000
const RECEIPT_POLL_INTERVAL_MS = 2_000

// No `BundlerMethod` entry for this one — it is not a method the send/estimate paths use, only
// the post-failure lookup below.
const ETH_GET_USER_OPERATION_BY_HASH = "eth_getUserOperationByHash"

/**
 * Node/undici codes for a failure in the connect phase — before any request bytes reached the
 * bundler, so nothing can have been submitted. Anything else (a reset mid-request, an abort, a
 * truncated/non-JSON body) leaves the op's fate unknown and is treated as ambiguous: guessing
 * "not submitted" there would resend the calls natively and execute them twice.
 */
const UNREACHABLE_CODES = new Set([
	"ECONNREFUSED",
	"ENOTFOUND",
	"EAI_AGAIN",
	"EHOSTUNREACH",
	"ENETUNREACH",
	// undici's own connect-phase timeout; distinct from an earlier-established socket stalling.
	"UND_ERR_CONNECT_TIMEOUT",
	// TLS certificate verification failures (OpenSSL X509 codes, plus Node's hostname-mismatch).
	"DEPTH_ZERO_SELF_SIGNED_CERT",
	"SELF_SIGNED_CERT_IN_CHAIN",
	"UNABLE_TO_GET_ISSUER_CERT",
	"UNABLE_TO_GET_ISSUER_CERT_LOCALLY",
	"UNABLE_TO_VERIFY_LEAF_SIGNATURE",
	"CERT_HAS_EXPIRED",
	"CERT_UNTRUSTED",
	"ERR_TLS_CERT_ALTNAME_INVALID",
])

type BundlerCallOutcome = "rejected" | "unreachable" | "ambiguous"

/**
 * A bundler JSON-RPC call that returned no result. `outcome` records how definite the failure
 * was, which the send path must not flatten: a `rejected` or `unreachable` call provably did not
 * enter the op, while an `ambiguous` one may have been accepted with only the response lost.
 */
class BundlerCallError extends Error {
	constructor(
		message: string,
		readonly outcome: BundlerCallOutcome,
		readonly reason?: unknown,
	) {
		super(message)
		this.name = "BundlerCallError"
	}
}

function connectFailureCode(error: unknown): string | undefined {
	const cause = (error as { cause?: { code?: unknown } } | undefined)?.cause
	return typeof cause?.code === "string" ? cause.code : undefined
}

function describeError(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}

/**
 * Whether a JSON-RPC code is a deterministic rejection of the request: a parse/request error, or
 * the ERC-4337 (ERC-7769) range the bundler uses for a definite refusal (which includes -32521,
 * the validation/inclusion rejection). Only these codes let the send path fall back natively.
 */
function isDeterministicRejectionCode(code: unknown): boolean {
	if (typeof code !== "number") return false
	// Parse error (-32700) and invalid request/params (-32600…-32602).
	if (code === -32700 || (code >= -32602 && code <= -32600)) return true
	// The ERC-4337 rejection range, -32599…-32500.
	return code >= -32599 && code <= -32500
}

/**
 * Submits self-initiated, paymaster-sponsored UserOperations through the bundler,
 * so the solver pays gas in stablecoins rather than native token. Shared by the
 * delegation setup and the vault sweep/redeem paths.
 *
 * Submission contract (so callers can safely fall back to a native tx without
 * risking a double-execution):
 * - Returns `null` when the op was **never submitted** — paymaster/bundler not
 *   configured, insufficient USDC, preparation failed (approve tx or signing), the
 *   bundler rejected `eth_sendUserOperation` outright, the connection could not be
 *   established, or a post-failure lookup proved the op never entered the pool. The
 *   caller may fall back to native.
 *   `null` means *this* op never entered the pool, not that nothing is pending for
 *   its EntryPoint nonce: a retry builds a new op on the same nonce, and the bundler's
 *   "replacement underpriced"/"already known" answer for it rejects the new op only.
 * - Returns `{ txHash, userOp }` once a receipt is observed.
 * - **Throws** {@link SubmittedUnconfirmedError} when the op **was submitted** but no
 *   receipt arrived, or when its submission could not be determined at all (the send
 *   response was lost and the lookup could not ask). The caller must NOT fall back (the
 *   op may still land) — retry on the next cycle, and resolve the op's fate later with
 *   {@link lookupUserOpReceipt}.
 */
export class UserOpSender {
	private logger: Logger

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly configService: FillerConfigService,
		private readonly signer: Signer,
	) {
		this.logger = moduleLogger(clientManager.loggers, "userop-sender")
	}

	/** True when a sponsored UserOp is even possible on this chain (paymaster + bundler configured). */
	canSponsor(chain: string): boolean {
		return (
			hasPaymaster(chain, this.configService) &&
			!!this.configService.getEntryPointAddress(chain) &&
			!!this.configService.getBundlerUrl(chain)
		)
	}

	async trySendSponsored(req: SponsoredUserOpRequest): Promise<{ txHash: HexString; userOp: SubmittedUserOp } | null> {
		const { chain, callData, eip7702Auth, nonceKey = 0n, gas, permitBootstrap } = req
		// A Permit2-mode op reports a validity deadline the paymaster enforces; a PERMIT bootstrap
		// op's EIP-2612 permit never expires. Only the former stops landing after its deadline.
		const expires = !permitBootstrap

		const entryPoint = this.configService.getEntryPointAddress(chain)
		const bundlerUrl = this.configService.getBundlerUrl(chain)
		if (!hasPaymaster(chain, this.configService) || !entryPoint || !bundlerUrl) {
			return null
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const walletClient = this.clientManager.getWalletClient(chain)
		const solverAccount = this.signer.address as HexString
		const chainId = this.configService.getChainId(chain)

		let pm: PaymasterDataResult
		let auth: Eip7702Authorization | undefined
		let fees: { maxFeePerGas: bigint; maxPriorityFeePerGas: bigint }
		try {
			// Fetched before paymaster selection so the deposit gate can price the op's
			// max prefund; a failure here means nothing was submitted, safe to fall back.
			fees = await this.getGasPrice(bundlerUrl, publicClient)

			// Without explicit limits the fallbacks (~1.9M gas) over-require the deposit;
			// a false skip degrades to the caller's native fallback, which is safe —
			// under-requiring would sign an op the bundler is bound to reject.
			const gasForPrefund = gas ?? {
				verificationGasLimit: FALLBACK_VERIFICATION_GAS_LIMIT,
				callGasLimit: FALLBACK_CALL_GAS_LIMIT,
				preVerificationGas: FALLBACK_PRE_VERIFICATION_GAS,
			}

			pm = await buildPaymasterAndData({
				chain,
				solverAccount,
				publicClient,
				walletClient,
				signer: this.signer,
				configService: this.configService,
				permitBootstrap,
				prefund: {
					baseGas:
						gasForPrefund.callGasLimit + gasForPrefund.verificationGasLimit + gasForPrefund.preVerificationGas,
					maxFeePerGas: fees.maxFeePerGas,
				},
				logger: this.logger,
			})
			if (pm.type === "none") {
				this.logger.warn(
					{ chain, solverAccount, reason: pm.reason },
					"No paymaster can sponsor this UserOp; caller should fall back to native",
				)
				return null
			}

			// Sign the EIP-7702 authorization only after paymaster data is built —
			// bootstrapping an approval (to Permit2 or the paymaster) sends a tx from the
			// authority EOA, and an authorization signed before it embeds a stale nonce.
			auth = typeof eip7702Auth === "function" ? await eip7702Auth() : eip7702Auth
		} catch (error) {
			// Preparation failures (approve tx, permit/authorization signing) happen
			// before anything reaches the bundler — never submitted, safe to fall back.
			this.logger.warn({ chain, error }, "Failed to prepare sponsored UserOp; caller should fall back to native")
			return null
		}
		this.logger.info({ chain, paymaster: pm.address, type: pm.type, token: pm.token }, "Paymaster selected")
		const paymasterAndData = pm.paymasterAndData
		const { maxFeePerGas, maxPriorityFeePerGas } = fees

		const nonce = (await publicClient.readContract({
			address: entryPoint,
			abi: ENTRYPOINT_ABI,
			functionName: "getNonce",
			args: [solverAccount, nonceKey],
		})) as bigint

		const formattedAuth = auth
			? {
					address: auth.address,
					chainId: toHex(auth.chainId),
					nonce: toHex(auth.nonce),
					r: auth.r,
					s: auth.s,
					yParity: toHex(auth.yParity),
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

		// The v0.8 EIP-712 digest of this typed data *is* the userOpHash — the exact payload
		// `buildSignedUserOp` signs — so we can know the hash the bundler should return without
		// asking it, which matters when the send response is lost.
		const localUserOpHash = hashTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, entryPoint, BigInt(chainId)),
		) as HexString

		// Timestamped at the attempt: a lost response has no time of its own to record.
		const submittedAt = Date.now()

		let userOpHash: HexString
		try {
			const sent = await this.sendBundlerRpc<HexString | null>(
				bundlerUrl,
				BundlerMethod.ETH_SEND_USER_OPERATION,
				[bundlerUserOp, entryPoint],
				BUNDLER_SIMULATION_TIMEOUT_MS,
			)
			// A valid answer is the op's hash. Anything else (a null `result` from a malformed
			// proxy response) is not an answer, so it falls into the ambiguous branch below.
			if (typeof sent !== "string") {
				throw new BundlerCallError(
					`Bundler returned no UserOp hash (${BundlerMethod.ETH_SEND_USER_OPERATION})`,
					"ambiguous",
				)
			}
			if (sent.toLowerCase() !== localUserOpHash.toLowerCase()) {
				this.logger.warn(
					{ chain, expected: localUserOpHash, received: sent },
					"Bundler returned a different UserOp hash than the signed op's; using the bundler's",
				)
			}
			userOpHash = sent
		} catch (error) {
			const outcome = error instanceof BundlerCallError ? error.outcome : "ambiguous"
			if (outcome !== "ambiguous") {
				// The bundler answered and refused the op, or the connection never reached it —
				// either way nothing entered the pool, safe to fall back.
				this.logger.warn(
					{ chain, outcome, error },
					`Bundler did not accept UserOp (${outcome}); caller should fall back to native`,
				)
				return null
			}

			// The request may have been accepted and only its response lost. Ask the bundler
			// whether it holds the op before deciding; falling back here is what would
			// double-execute the calls.
			const found = await this.findSubmittedOp(bundlerUrl, localUserOpHash)
			if (found === "absent") {
				// We did ask, and it does not hold the op. A residual race remains: a bundler
				// still validating the op when our request timed out could add it after our
				// last lookup, so the lookup delays exist to make that window small.
				this.logger.warn(
					{ chain, userOpHash: localUserOpHash },
					"Bundler's send response was lost and it does not hold the UserOp; caller should fall back to native",
				)
				return null
			}
			if (found === "unknown") {
				// Every lookup failed too, so we cannot tell whether the op was accepted. The
				// caller must not fall back — a resend could execute the calls twice.
				throw new SubmittedUnconfirmedError(
					`Sponsored UserOp ${localUserOpHash}: the bundler's answer was lost and the op could not be looked up`,
					chain,
					{ kind: "userOp", userOpHash: localUserOpHash, nonce, nonceKey, expires, submittedAt },
				)
			}
			// The bundler holds the op: proceed exactly as a successful send, from the locally
			// computed hash.
			this.logger.info({ chain, userOpHash: localUserOpHash }, "Bundler accepted UserOp after a lost response")
			userOpHash = localUserOpHash
		}

		this.logger.info({ chain, userOpHash }, "Sponsored UserOp submitted to bundler")

		const receipt = await this.waitForUserOpReceipt(bundlerUrl, userOpHash)
		const txHash = receipt?.receipt?.transactionHash
		if (!txHash) {
			// Submitted but unconfirmed — it may still land, so the caller must NOT
			// fall back to a native tx (that would double-execute). The nonce and key
			// travel with the error so a later recovery can read the EntryPoint nonce.
			throw new SubmittedUnconfirmedError(
				`Sponsored UserOp ${userOpHash} submitted but no receipt within timeout`,
				chain,
				{ kind: "userOp", userOpHash, nonce, nonceKey, expires, submittedAt },
			)
		}
		if (receipt?.success === false) {
			// Included but the inner execution reverted — surface it rather than report
			// success. The caller must not fall back (the op was mined).
			throw new Error(`Sponsored UserOp ${userOpHash} reverted on-chain (tx ${txHash})`)
		}

		this.logger.info({ chain, userOpHash, txHash }, "Sponsored UserOp confirmed — paymaster paid gas")
		// The op's identity travels with the receipt: a caller that stores it can still settle the
		// op by its EntryPoint nonce if the bundle's tx hash later vanishes in a reorg.
		return { txHash, userOp: { kind: "userOp", userOpHash, nonce, nonceKey, expires, submittedAt } }
	}

	/**
	 * One `eth_getUserOperationReceipt` for an op this filler submitted. Null when the bundler
	 * does not know it — the op has not been indexed, which is not proof it never landed. Throws
	 * when the bundler could not be asked, so a caller can tell "not found" apart from "couldn't ask".
	 */
	async lookupUserOpReceipt(
		chain: string,
		userOpHash: HexString,
	): Promise<{ txHash: HexString; success: boolean } | null> {
		const bundlerUrl = this.configService.getBundlerUrl(chain)
		if (!bundlerUrl) throw new Error(`No bundler is configured for ${chain}`)
		const receipt = await this.sendBundlerRpc<{ success?: boolean; receipt?: { transactionHash?: HexString } } | null>(
			bundlerUrl,
			BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT,
			[userOpHash],
		)
		if (!receipt) return null
		const txHash = receipt.receipt?.transactionHash
		// A receipt with no transaction hash cannot be acted on; the caller must not read it as "not found".
		if (!txHash) throw new Error(`Bundler returned a receipt for ${userOpHash} with no transaction hash`)
		// Mirrors the send path: only `success === false` means the inner execution reverted.
		return { txHash, success: receipt.success !== false }
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
			}>(
				p.bundlerUrl,
				BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS,
				[bundlerUserOp, p.entryPoint],
				BUNDLER_SIMULATION_TIMEOUT_MS,
			)

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
		// The v0.8 userOpHash is the EIP-712 digest of the operation, so signing the typed
		// data yields the same signature while keeping the payload visible to the signing
		// backend (e.g. Turnkey's policy engine) instead of an opaque 32-byte digest.
		userOp.signature = await this.signer.signTypedData(
			CryptoUtils.packedUserOpTypedData(userOp, p.entryPoint, BigInt(p.chainId)),
		)
		return userOp
	}

	/**
	 * Prices the op for the rundler bundler, as the fill path does: its suggested fees, or the
	 * chain's gas price raised to the priority fee it requires where it suggests none.
	 */
	private async getGasPrice(
		bundlerUrl: string,
		publicClient: PublicClient,
	): Promise<{ maxFeePerGas: bigint; maxPriorityFeePerGas: bigint }> {
		const [gasPrice, latestBlock] = await Promise.all([
			publicClient.getGasPrice(),
			publicClient.getBlock({ blockTag: "latest" }),
		])
		return rundlerUserOperationFees(
			bundlerUrl,
			{
				maxFeePerGas: gasPrice + (gasPrice * MAX_FEE_BUMP_PERCENT) / 100n,
				maxPriorityFeePerGas: gasPrice + (gasPrice * PRIORITY_FEE_BUMP_PERCENT) / 100n,
			},
			{
				baseFeePerGas: latestBlock.baseFeePerGas ?? gasPrice,
				priorityFeeBumpPercent: PRIORITY_FEE_BUMP_PERCENT,
				maxFeeBumpPercent: MAX_FEE_BUMP_PERCENT,
			},
		)
	}

	/**
	 * One bundler JSON-RPC call, used by every request in this class. `timeoutMs` bounds the
	 * request; a failure is thrown as a {@link BundlerCallError} whose `outcome` says whether it
	 * is safe to treat the call as never having reached the bundler — the distinction the send
	 * path relies on to avoid a native resend of an op that may already be submitted.
	 */
	private async sendBundlerRpc<T>(
		bundlerUrl: string,
		method: string,
		params: unknown[],
		timeoutMs = BUNDLER_POLL_TIMEOUT_MS,
	): Promise<T> {
		let response: Response
		try {
			response = await fetch(bundlerUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
				signal: AbortSignal.timeout(timeoutMs),
			})
		} catch (error) {
			// `fetch` rejects only when the request never completed. A connect-phase failure (no
			// bytes sent) is unreachable; the abort timeout, a socket reset mid-flight or a TLS
			// failure after the handshake is ambiguous — the op may have been accepted and only
			// the response lost.
			const code = connectFailureCode(error)
			const outcome: BundlerCallOutcome = code && UNREACHABLE_CODES.has(code) ? "unreachable" : "ambiguous"
			throw new BundlerCallError(
				`Bundler request failed (${method})${code ? ` [${code}]` : ""}: ${describeError(error)}`,
				outcome,
				error,
			)
		}

		let body: { result?: T; error?: unknown }
		try {
			body = (await response.json()) as { result?: T; error?: unknown }
		} catch (error) {
			// A non-JSON body (an HTML proxy error page, a truncated stream) says nothing about
			// whether the op was accepted.
			throw new BundlerCallError(`Bundler returned a non-JSON response (${method})`, "ambiguous", error)
		}

		const rpcError = body?.error
		if (rpcError !== undefined && rpcError !== null) {
			if (typeof rpcError === "object") {
				const message = (rpcError as { message?: string }).message ?? JSON.stringify(rpcError)
				// Only an HTTP 200 carrying a deterministic rejection code is a definite refusal.
				// A proxy's JSON 502/504 body carries an object error too, but it forwarded the
				// request; rundler's -32603 "internal error" can also come after the pool admitted
				// the op. Both are ambiguous — reading them as rejected would resend natively. A
				// missing or non-numeric code, or any code outside the parse/rejection ranges, is
				// likewise ambiguous: when in doubt, keep the op.
				const rejected =
					response.status === 200 && isDeterministicRejectionCode((rpcError as { code?: unknown }).code)
				throw new BundlerCallError(
					`Bundler RPC error (${method}): ${message}`,
					rejected ? "rejected" : "ambiguous",
				)
			}
			// A non-object `error` (a proxy's plain `{"error":"Bad Gateway"}`) is not a JSON-RPC
			// answer, so it stays ambiguous.
			throw new BundlerCallError(
				`Bundler returned a malformed error (${method}): ${JSON.stringify(rpcError)}`,
				"ambiguous",
			)
		}
		// A `null` result is a real answer (e.g. "this op is unknown"); only a missing `result`
		// key, or an HTTP error with no JSON-RPC body, is ambiguous.
		if (body?.result !== undefined) return body.result
		throw new BundlerCallError(`Bundler returned no result (${method}): HTTP ${response.status}`, "ambiguous")
	}

	/**
	 * After an ambiguous `eth_sendUserOperation`, asks the bundler whether it actually holds the
	 * op. Rundler's `eth_getUserOperationByHash` checks its mempool and then the chain, so a
	 * non-null result means the op was accepted; a definite `null` on every attempt means it
	 * never entered the pool.
	 *
	 * `lookupDelaysMs` are inter-attempt waits, so the default `[2_000, 5_000, 10_000]` looks up
	 * at about 2 s, 7 s and 17 s after the failed send.
	 *
	 * Returns `"unknown"` when any attempt failed, because that is not proof of absence: a
	 * bundler still validating the op after our request timed out could add it after the last
	 * attempt, and the widening delays exist to shrink that window. Only `"absent"` — a `null`
	 * from every attempt — lets the caller fall back.
	 */
	private async findSubmittedOp(
		bundlerUrl: string,
		userOpHash: HexString,
		lookupDelaysMs: readonly number[] = [2_000, 5_000, 10_000],
	): Promise<"found" | "absent" | "unknown"> {
		let everyAttemptAnswered = true
		for (const delayMs of lookupDelaysMs) {
			await new Promise((resolve) => setTimeout(resolve, delayMs))
			try {
				const op = await this.sendBundlerRpc<unknown>(bundlerUrl, ETH_GET_USER_OPERATION_BY_HASH, [userOpHash])
				if (op) return "found"
			} catch {
				// No definite answer this time; the next attempt may reach the bundler.
				everyAttemptAnswered = false
			}
		}
		return everyAttemptAnswered ? "absent" : "unknown"
	}

	/**
	 * Polls for the op's receipt until `deadlineMs` has elapsed, 2 s apart. Time-based so the
	 * wait cannot stretch with the per-poll timeout; `deadlineMs` and `intervalMs` stay
	 * overridable for tests.
	 */
	private async waitForUserOpReceipt(
		bundlerUrl: string,
		userOpHash: HexString,
		deadlineMs = RECEIPT_WAIT_DEADLINE_MS,
		intervalMs = RECEIPT_POLL_INTERVAL_MS,
	): Promise<{ success?: boolean; receipt?: { transactionHash: HexString } } | null> {
		const deadline = Date.now() + deadlineMs
		for (;;) {
			try {
				const receipt = await this.sendBundlerRpc<{
					success?: boolean
					receipt: { transactionHash: HexString }
				} | null>(bundlerUrl, BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT, [userOpHash])
				if (receipt) return receipt
			} catch {
				// Not indexed yet.
			}
			if (Date.now() >= deadline) return null
			await new Promise((resolve) => setTimeout(resolve, intervalMs))
		}
	}
}
