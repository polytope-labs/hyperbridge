import type { HexString } from "@hyperbridge/sdk"
import { encodeFunctionData, erc20Abi, formatEther, maxUint256, zeroAddress } from "viem"
import { encodeERC7821ExecuteBatch } from "@hyperbridge/sdk"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { type Logger, moduleLogger } from "./Logger"
import type { Signer } from "./wallet"
import { hasPaymaster } from "./paymaster"
import { resolvePendingPermit2Approval } from "./paymaster/provider/simplex"
import { PendingOpOnNonceError, SubmittedUnconfirmedError, type SubmittedUserOp } from "./submission"
import { UserOpSender, type Eip7702Authorization } from "./UserOpSender"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/**
 * Fixed gas limit for set-code (0x04) txs.
 */
export const DELEGATION_TX_GAS_FLOOR = 650_000n

/** Extra gas for an ERC-20 approve folded into the delegation tx. */
export const BATCHED_APPROVE_GAS = 60_000n

/** callGasLimit for an op carrying `approve(Permit2, max)` through ERC-7821. */
const BOOTSTRAP_CALL_GAS_LIMIT = 150_000n

/**
 * How long to wait for a sponsored delegation's tx receipt on this chain's public RPC.
 * The bundler's node saw the tx first, so the public one can lag; past this bound the
 * delegation is reported "failed" with no native fallback and the caller retries.
 */
const BUNDLER_RECEIPT_WAIT_TIMEOUT_MS = 30_000

/**
 * The ERC-7821 batch that installs the Permit2 allowance. Shared by the first-time delegation
 * op and by {@link DelegationService.ensurePermit2Allowance}, so the two carriers can never
 * drift into approving different amounts or spenders.
 */
function bootstrapCallData(approval: { token: HexString; spender: HexString }): HexString {
	return encodeERC7821ExecuteBatch([
		{
			target: approval.token,
			value: 0n,
			data: encodeFunctionData({
				abi: erc20Abi,
				functionName: "approve",
				args: [approval.spender, maxUint256],
			}),
		},
	])
}

/** What one attempt to delegate through the bundler left behind. */
type BundlerDelegationOutcome = "delegated" | "unavailable" | "pending" | "failed"

/** The two delegation operations, for the lock below. */
type DelegationOperation = "setup" | "revoke"

interface InFlightDelegation {
	operation: DelegationOperation
	attempt: Promise<boolean>
}

/**
 * One delegation operation per wallet and chain at a time, shared by every
 * `DelegationService` in the process.
 *
 * Two services delegate the same wallet — the filler's and the limit-order service's —
 * and nothing serialises them against each other: an "Enable fills" click racing a
 * limit-order post would send two ops on the same EntryPoint nonce. The bundler would
 * refuse the second as pending on the nonce, so it would add nothing but a failed attempt.
 * The map is module-level rather than per instance because the two are separate
 * instances and must still see each other's attempt.
 */
const inFlightDelegationByWallet = new Map<string, InFlightDelegation>()

/** The identity of a sponsored delegation op that was submitted but never confirmed. */
type UnconfirmedDelegationOp = Pick<SubmittedUserOp, "userOpHash" | "nonce" | "nonceKey" | "submittedAt">

/**
 * How long a submitted delegation op must have gone unconfirmed before a definite "the bundler
 * does not know it" counts as a drop. Covers the bundler's own latency in pooling and indexing
 * an op it just accepted.
 */
const UNCONFIRMED_OP_GRACE_MS = 2 * 60_000

/**
 * The sponsored delegation op each wallet and chain is still waiting on, by the same key as
 * {@link inFlightDelegationByWallet}. The in-flight map only covers an attempt while it runs;
 * an op that outlived its receipt wait may still be in the bundler's pool afterwards. A retry
 * (a fills enable, a limit-order create, a restart of the caller) resolves the earlier op by
 * its receipt, the EntryPoint nonce or a bundler lookup, instead of building a new op on its
 * nonce, which the bundler would refuse as still pending.
 * Module-level for the same reason as the map above.
 */
const unconfirmedDelegationOps = new Map<string, UnconfirmedDelegationOp>()

/**
 * Service for managing EIP-7702 delegation of the filler's EOA to the SolverAccount contract.
 * This enables the filler to participate in solver selection mode.
 *
 * When the Simplex paymaster is configured and the filler holds stablecoins already
 * approved to Permit2, delegation is performed via a no-op UserOp sent through the
 * bundler — the paymaster pays gas in stablecoins. Falls back to a direct type-0x04 tx
 * if the bundler path is unavailable.
 *
 * The paymaster prefunds through Permit2, which needs a standing token approval to the
 * Permit2 contract. On a chain the solver has never used, the delegation op installs
 * that approval itself and pays for the privilege with an EIP-2612 permit, so a
 * permit-capable fee token bootstraps with no native at all. A token without a permit
 * (BNB Chain) still needs native dust once, for the type-0x04 tx that batches the
 * approve in. Every op after that is native-free either way.
 */
export class DelegationService {
	private logger: Logger
	private readonly userOpSender: UserOpSender

	constructor(
		private clientManager: ChainClientManager,
		private configService: FillerConfigService,
		private signer: Signer,
	) {
		this.logger = moduleLogger(clientManager.loggers, "delegation-service")
		this.userOpSender = new UserOpSender(clientManager, configService, signer)
	}

	/**
	 * Signs the authorization tuple and reports the nonce a direct set-code tx must carry.
	 *
	 * The tx nonce and the tuple nonce are two different counters, and the tuple's is the
	 * one *after* the tx's: EIP-7702 compares the tuple's nonce against the authority's
	 * nonce once the tx has bumped it. A direct tx therefore reads `pending` — what viem
	 * would fill anyway — and signs the tuple at `nonce + 1`. Reading `latest` instead
	 * signs one too low whenever an EOA tx is in flight, and a mismatched tuple is skipped
	 * without reverting: the receipt reads success while the account stays undelegated.
	 *
	 * @param viaBundler When true, the bundler submits the tx, so the tuple uses the
	 *                   authority's current nonce. When false, the EOA sends the type-0x04
	 *                   tx itself and the tuple is signed at that tx's nonce + 1.
	 */
	private async buildAuthorization(
		chain: string,
		contractAddress: HexString,
		viaBundler = false,
	): Promise<{ authorization: Eip7702Authorization; txNonce: number }> {
		const publicClient = this.clientManager.getPublicClient(chain)
		const chainId = this.configService.getChainId(chain)
		const authorityAddress = this.signer.address as HexString

		let txNonce: number
		if (viaBundler) {
			// The bundler submits the op from its own EOA, so there is no nonce for this
			// wallet to claim — but the tuple must still be the one the authority will have
			// when the op executes. A tx in flight would consume that nonce the moment it
			// mines, leaving the tuple stale and the op's delegation skipped, so refuse
			// until it has landed.
			const [latest, pending] = await Promise.all([
				publicClient.getTransactionCount({ address: authorityAddress, blockTag: "latest" }),
				publicClient.getTransactionCount({ address: authorityAddress, blockTag: "pending" }),
			])
			if (pending > latest) {
				throw new Error(
					`The wallet has a transaction in flight on ${chain}; the delegation is signed once it has landed`,
				)
			}
			txNonce = latest
		} else {
			txNonce = await publicClient.getTransactionCount({ address: authorityAddress, blockTag: "pending" })
		}
		const authorizationNonce = viaBundler ? txNonce : txNonce + 1

		// The signer owns the encoding: a backend with structured 7702 support keeps
		// the tuple inspectable by its policy engine, one without hashes it itself.
		const { r, s, yParity } = await this.signer.signAuthorization({
			chainId,
			contractAddress,
			nonce: Number(authorizationNonce),
		})
		// EIP-7702 skips an invalid tuple without reverting — the receipt reads
		// success while the account stays undelegated — so an out-of-range parity
		// from a custom signer must fail here, loudly, not on-chain, silently.
		if (yParity !== 0 && yParity !== 1) {
			throw new Error(
				`Signer returned yParity ${yParity} for the EIP-7702 authorization; expected 0 or 1 ` +
					"(a backend returning the legacy v should subtract 27)",
			)
		}

		return {
			authorization: { chainId, address: contractAddress, nonce: authorizationNonce, r, s, yParity },
			txNonce,
		}
	}

	private async sendDelegationTransaction(
		chain: string,
		authorization: Eip7702Authorization,
		nonce: number,
		approval?: { token: HexString; spender: HexString } | null,
	): Promise<HexString> {
		const walletClient = this.clientManager.getWalletClient(chain)

		// Every backend goes out the same way: viem prepares the set-code tx and the
		// signer signs it. A backend whose transaction API cannot express an
		// authorization list handles that in its own `signTransaction` — see
		// `mpcVaultSigner` — not here.
		//
		// When an approval is folded in, the tx calls `token.approve(spender, max)` while
		// carrying the authorization: the delegation still lands (the authorization list is
		// processed independently of `to`), and the approve executes with the EOA as sender.
		// Otherwise it is a no-op self-call whose only payload is the authorization.
		//
		// `nonce` is passed explicitly: the tuple was signed against it (at `nonce + 1`), so
		// letting viem fill another one from `pending` could desync the two.
		if (approval) {
			return (await walletClient.sendTransaction({
				to: approval.token,
				value: 0n,
				data: encodeFunctionData({
					abi: erc20Abi,
					functionName: "approve",
					args: [approval.spender, maxUint256],
				}),
				authorizationList: [authorization],
				chain: walletClient.chain,
				gas: DELEGATION_TX_GAS_FLOOR + BATCHED_APPROVE_GAS,
				nonce,
			})) as HexString
		}

		return (await walletClient.sendTransaction({
			to: this.signer.address,
			value: 0n,
			authorizationList: [authorization],
			chain: walletClient.chain,
			gas: DELEGATION_TX_GAS_FLOOR,
			nonce,
		})) as HexString
	}

	/**
	 * Best-effort resolution of a Permit2 approval to fold into a native delegation. Never
	 * throws: a resolution failure (RPC error, misconfig) yields null and the delegation
	 * proceeds as a plain self-call.
	 */
	private async resolvePendingPermit2Approval(
		chain: string,
	): Promise<{ token: HexString; spender: HexString; permitCapable: boolean } | null> {
		try {
			const paymaster = this.configService.getSimplexPaymasterAddress(chain)
			if (!paymaster) return null
			return await resolvePendingPermit2Approval(
				this.clientManager.getPublicClient(chain),
				this.signer.address as HexString,
				paymaster,
				chain,
				this.configService,
			)
		} catch (error) {
			this.logger.warn({ chain, error }, "Could not resolve a Permit2 approval to batch into delegation")
			return null
		}
	}

	/**
	 * Checks if the filler's EOA is already delegated to the SolverAccount contract on a specific chain.
	 *
	 * Best-effort: callers retry, so a read failure answers "not delegated".
	 */
	async isDelegated(chain: string): Promise<boolean> {
		try {
			return await this.readDelegation(chain)
		} catch (error) {
			this.logger.error({ chain, error }, "Failed to check delegation status")
			return false
		}
	}

	/**
	 * Whether the EOA's code is the EIP-7702 delegation to the configured SolverAccount.
	 *
	 * A `blockNumber` makes this a claim about one mined block, so a read failure throws
	 * rather than answering "not delegated"; `confirmDelegationAt` turns that into "not
	 * confirmed". Without one it is a best-effort check, and `isDelegated` swallows a
	 * read failure to false.
	 */
	private async readDelegation(chain: string, blockNumber?: bigint): Promise<boolean> {
		const client = this.clientManager.getPublicClient(chain)
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			return false
		}

		const code = await client.getCode({
			address: this.signer.address,
			...(blockNumber !== undefined ? { blockNumber } : {}),
		})

		if (!code || code === "0x") {
			return false
		}

		if (!code.toLowerCase().startsWith(DELEGATION_INDICATOR_PREFIX)) {
			return false
		}

		const delegatedTo = ("0x" + code.slice(8)) as HexString
		const isCorrectDelegate = delegatedTo.toLowerCase() === solverAccountContract.toLowerCase()

		this.logger.debug(
			{ chain, delegatedTo, expected: solverAccountContract, isCorrectDelegate },
			"Checked delegation status",
		)

		return isCorrectDelegate
	}

	/**
	 * Confirms the delegation at the block a tx mined in. A failed read of that one block is
	 * not a verdict, so the wallet's delegation at `latest` decides instead: a delegation that
	 * landed is not reported as failed because a node could not serve a historical `getCode`.
	 * `isDelegated` answers false when `latest` cannot be read either, which is "not confirmed",
	 * never a pass.
	 */
	private async confirmDelegationAt(chain: string, blockNumber: bigint): Promise<boolean> {
		try {
			return await this.readDelegation(chain, blockNumber)
		} catch (error) {
			this.logger.error(
				{ chain, blockNumber, error },
				"Could not confirm delegation at the mined block; checking the latest state",
			)
			return this.isDelegated(chain)
		}
	}

	/**
	 * Sets up EIP-7702 delegation via the bundler. Uses the Simplex paymaster when
	 * configured and the filler holds a sufficient stablecoin balance.
	 *
	 * The op is a no-op once the account has a Permit2 allowance. On a chain it has
	 * never used, and only when the fee token implements EIP-2612 (`permitCapable`),
	 * the op instead carries `approve(Permit2, max)` as its callData and asks the
	 * paymaster for PERMIT mode: the permit covers this op's gas, and the op installs
	 * the allowance every later op authorizes against. That is the whole zero-native
	 * bootstrap — one sponsored op, after which the account never needs a permit again.
	 *
	 * Returns an outcome rather than a boolean because only "unavailable" may fall back to
	 * a native tx: once the op reached the bundler it may still land, and a native retry
	 * alongside it would delegate twice.
	 */
	private async setupDelegationViaBundler(
		chain: string,
		pendingApproval?: { token: HexString; spender: HexString; permitCapable: boolean } | null,
	): Promise<BundlerDelegationOutcome> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract || !this.userOpSender.canSponsor(chain)) {
			this.logger.warn({ chain }, "Missing config for bundler-based delegation, falling back to direct tx")
			return "unavailable"
		}

		// Read before the try: nothing has been submitted yet, so a read failure must reach
		// the caller (which retries) rather than be read as a post-submit "failed", which
		// would never fall back.
		//
		// Fixed limits for the no-op delegation op — bundler estimation of EIP-7702
		// ops is unreliable (Alchemy echoes the input limits rather than simulating).
		//
		// A FRESH delegation (EOA has no code) burns far more verification gas on
		// first-time cold storage, so the proven 150k account limit clears rundler's
		// verification-efficiency policy. A RE-delegation (EOA already delegated) uses
		// much less (warm slots) — actual ~96k — so a loose limit falls below the 0.4
		// floor (`actual / (accountVerif + paymasterVerif)`). Tighten the account
		// verification limit for that case so the ratio clears 0.4 while still covering
		// usage.
		//
		// Only the account side is tuned here: the paymaster packs its own limit
		// (permit2VerificationGasLimit: 200k, 300k on Polygon), charged to the paymaster frame.
		const code = await this.clientManager.getPublicClient(chain).getCode({
			address: this.signer.address as HexString,
		})
		const isFreshEoa = !code || code === "0x"

		try {
			this.logger.info(
				{ chain, solverAccount: this.signer.address, solverAccountContract, mode: "bundler" },
				"Setting up EIP-7702 delegation via bundler with paymaster",
			)

			// Bootstrap only when the token can pay for it: without a permit the paymaster
			// would have to land a native-funded approve first, and that same approve would
			// then make this op's callData a non-zero → non-zero change the USDT rule
			// rejects. Those chains stay on the no-op op and the native fallbacks.
			const bootstrap = !!pendingApproval?.permitCapable
			const callData = bootstrap ? bootstrapCallData(pendingApproval!) : ("0x" as HexString)

			// The EIP-7702 authorization rides inside the UserOp so a not-yet-delegated
			// EOA is delegated in the op (bundler submits the tx, so it uses the current
			// nonce). Passed as a factory: the sender signs it only after paymaster data
			// is built, because a first-time approval (to Permit2 or the paymaster) sends
			// a tx from this same EOA at that step, invalidating an earlier authorization.
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData,
				eip7702Auth: async () => (await this.buildAuthorization(chain, solverAccountContract, true)).authorization,
				gas: {
					verificationGasLimit: isFreshEoa ? 150_000n : 80_000n,
					// The no-op needs almost nothing; the bootstrap runs an approve through
					// ERC-7821 `execute`, a cold SSTORE plus dispatch overhead.
					callGasLimit: bootstrap ? BOOTSTRAP_CALL_GAS_LIMIT : 50_000n,
					preVerificationGas: 100_000n,
				},
				permitBootstrap: bootstrap,
			})

			if (result) {
				// A receipt is not proof the delegation applied. A tuple skipped for a stale
				// nonce, or one signed for an OLD SolverAccount after an upgrade, still lets
				// the op validate — against the old delegate — and reads as success. The code
				// at the mined block is the only proof.
				if (!(await this.confirmBundlerDelegation(chain, result.txHash))) {
					this.logger.error(
						{ chain, txHash: result.txHash },
						"Sponsored delegation mined but the wallet is not delegated to the SolverAccount",
					)
					return "failed"
				}
				this.logger.info(
					{ chain, txHash: result.txHash },
					"Delegation via bundler successful — paymaster paid gas",
				)
				unconfirmedDelegationOps.delete(this.walletKey(chain))
				return "delegated"
			}

			// null → the op was never submitted (no paymaster/bundler, insufficient USDC,
			// or the bundler rejected it). Safe to fall back to a direct tx.
			this.logger.warn({ chain }, "Sponsored delegation unavailable, falling back to direct tx")
			return "unavailable"
		} catch (error) {
			// Never fall back after a throw: the op was mined ("reverted on-chain") or the
			// sender failed outside its preparation section (e.g. the EntryPoint `getNonce`
			// read), and either way a native tx now could delegate a second time. A cheap
			// retry on the next call is not worth that.
			const delegated = await this.isDelegated(chain)
			if (error instanceof SubmittedUnconfirmedError || error instanceof PendingOpOnNonceError) {
				// The op was, or may have been, submitted, or an earlier op from this wallet
				// holds the nonce, so one of them may still land. Only a delegation already
				// visible on chain proves it did; otherwise this is pending and the caller
				// retries on the next cycle.
				this.logger.warn({ chain, error, delegated }, "Sponsored delegation unconfirmed; not falling back")
				// Remembered so the retry resolves this op before building another on its nonce.
				if (!delegated && error instanceof SubmittedUnconfirmedError && error.submission.kind === "userOp") {
					const { userOpHash, nonce, nonceKey, submittedAt } = error.submission
					unconfirmedDelegationOps.set(this.walletKey(chain), { userOpHash, nonce, nonceKey, submittedAt })
				}
				return delegated ? "delegated" : "pending"
			}
			this.logger.warn({ chain, error, delegated }, "Bundler delegation did not confirm; not falling back")
			return delegated ? "delegated" : "failed"
		}
	}

	/**
	 * Resolves the sponsored delegation op an earlier attempt left unconfirmed, before anything
	 * new is built on its nonce.
	 *
	 * - Its receipt is found: the delegation is confirmed on chain.
	 * - The EntryPoint nonce at `latest` has moved past the op's and the wallet is not delegated:
	 *   the nonce was consumed without delegating, so the op is gone.
	 * - The nonce has not moved, but the bundler definitely no longer holds the op and it is past
	 *   the grace period (see {@link DelegationService.bundlerDroppedOp}): it was dropped, so "gone".
	 * - Anything else, including a lookup or read that failed: still unresolved, so "pending".
	 *   The op may land, and a replacement or a native delegation alongside it would double up.
	 */
	private async resolveUnconfirmedOp(chain: string): Promise<"none" | "delegated" | "gone" | "failed" | "pending"> {
		const key = this.walletKey(chain)
		const op = unconfirmedDelegationOps.get(key)
		if (!op) return "none"

		let receiptLookupAnswered = false
		try {
			const receipt = await this.userOpSender.lookupUserOpReceipt(chain, op.userOpHash)
			if (receipt) {
				unconfirmedDelegationOps.delete(key)
				return (await this.confirmBundlerDelegation(chain, receipt.txHash)) ? "delegated" : "failed"
			}
			receiptLookupAnswered = true
		} catch (error) {
			// The bundler could not be asked; the nonce below is the other evidence.
			this.logger.warn({ chain, userOpHash: op.userOpHash, error }, "Could not look up the unconfirmed delegation op")
		}

		try {
			const entryPoint = this.configService.getEntryPointAddress(chain)
			if (!entryPoint) throw new Error(`No EntryPoint is configured for ${chain}`)
			const current = (await this.clientManager.getPublicClient(chain).readContract({
				address: entryPoint,
				abi: ENTRYPOINT_ABI,
				functionName: "getNonce",
				args: [this.signer.address as HexString, op.nonceKey],
				blockTag: "latest",
			})) as bigint
			if (current > op.nonce) {
				// The nonce was consumed. A read failure here throws rather than reading as
				// "not delegated", which would call a landed op gone.
				const delegated = await this.readDelegation(chain)
				unconfirmedDelegationOps.delete(key)
				return delegated ? "delegated" : "gone"
			}
			// The nonce has not moved, so the op is either still pooled or dropped without a
			// trace: a bundler that evicts it never consumes the nonce, and without this check
			// the record would stay "pending" until a restart, refusing every retry.
			if (receiptLookupAnswered && (await this.bundlerDroppedOp(chain, op))) {
				unconfirmedDelegationOps.delete(key)
				return "gone"
			}
		} catch (error) {
			this.logger.warn({ chain, userOpHash: op.userOpHash, error }, "Could not read the unconfirmed delegation op's nonce")
		}
		return "pending"
	}

	/**
	 * Whether the bundler has definitely dropped an unconfirmed delegation op. Rundler's
	 * `eth_getUserOperationByHash` checks its mempool and then the chain, so a definite `null`
	 * means the op is neither pooled nor landed. That only counts once the op is older than
	 * {@link UNCONFIRMED_OP_GRACE_MS}, so a lookup racing the bundler's own indexing is not read
	 * as a drop, and a failed or ambiguous lookup is not an answer at all.
	 *
	 * A wrong "gone" costs at most one set-code transaction's gas: once a native set-code tx
	 * lands it consumes the EOA nonce the old op's 7702 authorization was signed for, so the
	 * old op can no longer delegate. And "gone" only falls through to the normal setup, which
	 * tries a sponsored op first.
	 */
	private async bundlerDroppedOp(chain: string, op: UnconfirmedDelegationOp): Promise<boolean> {
		if (Date.now() - op.submittedAt < UNCONFIRMED_OP_GRACE_MS) return false
		try {
			return !(await this.userOpSender.isUserOpKnown(chain, op.userOpHash))
		} catch (error) {
			this.logger.warn({ chain, userOpHash: op.userOpHash, error }, "Could not ask the bundler whether it still holds the delegation op")
			return false
		}
	}

	/**
	 * Confirms the delegation at the block a sponsored op's transaction mined in. The public
	 * RPC can lag the bundler's, so the receipt is waited for, bounded by
	 * {@link BUNDLER_RECEIPT_WAIT_TIMEOUT_MS}. A receipt that cannot be read is not evidence
	 * either way, so the wallet's delegation at `latest` decides instead: a delegation that
	 * landed is not reported as failed because the receipt wait timed out.
	 */
	private async confirmBundlerDelegation(chain: string, txHash: HexString): Promise<boolean> {
		const publicClient = this.clientManager.getPublicClient(chain)
		try {
			const receipt = await publicClient.waitForTransactionReceipt({
				hash: txHash,
				timeout: BUNDLER_RECEIPT_WAIT_TIMEOUT_MS,
			})
			if (!receipt) {
				this.logger.error({ chain, txHash }, "The sponsored delegation tx has no receipt to confirm against")
				return this.isDelegated(chain)
			}
			return await this.confirmDelegationAt(chain, receipt.blockNumber)
		} catch (error) {
			this.logger.error({ chain, txHash, error }, "Could not read the sponsored delegation tx receipt")
			return this.isDelegated(chain)
		}
	}

	/**
	 * Installs `approve(Permit2, max)` on an account that is ALREADY delegated, using the same
	 * permit-funded op the first-time delegation carries — minus the authorization, which it
	 * does not need.
	 *
	 * This exists because delegation and bootstrap are separate facts. An account delegated by
	 * an earlier release has no Permit2 allowance (that release charged EIP-2612 permits and
	 * never touched Permit2), and `setupDelegation` returns before the bootstrap once
	 * `isDelegated` holds. Leaving it there strands every upgraded solver on the native-funded
	 * approve path.
	 *
	 * Best-effort and never throws: on failure the first sponsored op still falls back to
	 * `sendFundedApprove`, which works for a solver holding native.
	 */
	private async ensurePermit2Allowance(chain: string): Promise<void> {
		if (!this.userOpSender.canSponsor(chain)) return

		const pending = await this.resolvePendingPermit2Approval(chain)
		if (!pending) return
		if (!pending.permitCapable) {
			// No permit means no way to pay for this op without native, and a solver that holds
			// native can let the first sponsored op send the approve itself.
			this.logger.info(
				{ chain, token: pending.token },
				"Fee token has no permit; its Permit2 approval defers to the first sponsored op's funded approve",
			)
			return
		}

		this.logger.info(
			{ chain, token: pending.token },
			"Installing the Permit2 allowance with a permit-funded UserOp",
		)
		try {
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData: bootstrapCallData(pending),
				// No authorization and warm account storage, so the account side is the
				// re-delegation figure; the paymaster packs its own PERMIT limit.
				gas: {
					verificationGasLimit: 80_000n,
					callGasLimit: BOOTSTRAP_CALL_GAS_LIMIT,
					preVerificationGas: 100_000n,
				},
				permitBootstrap: true,
			})
			if (result) {
				this.logger.info({ chain, txHash: result.txHash }, "Permit2 allowance installed — paymaster paid gas")
				return
			}
			this.logger.warn(
				{ chain },
				"Permit-funded Permit2 approval unavailable; the first sponsored op will need a native-funded approve",
			)
		} catch (error) {
			this.logger.warn(
				{ chain, error },
				"Permit-funded Permit2 approval failed; the first sponsored op will need a native-funded approve",
			)
		}
	}

	/**
	 * Sets up EIP-7702 delegation from the filler's EOA to the SolverAccount contract.
	 *
	 * A fee token with no Permit2 allowance yet cannot be charged through Permit2, so the
	 * approve has to be installed by the very first operation. Which carrier does that
	 * depends on the token:
	 *
	 * - EIP-2612 token: the bundler goes first and the op pays for itself with a permit
	 *   while carrying the approve in its callData. No native is spent, so this is
	 *   preferred even on an EOA that holds some.
	 * - No permit (BSC pegged stables): only a native type-0x04 tx can carry the approve,
	 *   so that goes first when the EOA covers it.
	 *
	 * With nothing pending — the allowance is already in place — the bundler path goes
	 * first as usual. A plain direct tx is the last fallback in every case.
	 *
	 * One operation runs per wallet and chain at a time. Two services delegate the same
	 * wallet (the filler's and the limit-order service's, see
	 * `inFlightDelegationByWallet`), so a second call joins the attempt already in flight
	 * instead of sending a competing op.
	 */
	async setupDelegation(chain: string): Promise<boolean> {
		return this.withDelegationLock(chain, "setup", () => this.setupDelegationOnce(chain))
	}

	/** The key every per-wallet, per-chain delegation map shares. */
	private walletKey(chain: string): string {
		return `${this.signer.address.toLowerCase()}:${chain}`
	}

	/**
	 * Runs one delegation operation under the per-wallet/chain lock.
	 *
	 * A second call of the same operation joins the one in flight (they would do the same
	 * thing). A different operation — a revoke during a setup — queues behind it so the
	 * two never overlap, but still runs: returning the other operation's result would
	 * report a revoke that never happened.
	 */
	private withDelegationLock(
		chain: string,
		operation: DelegationOperation,
		work: () => Promise<boolean>,
	): Promise<boolean> {
		const key = this.walletKey(chain)
		const running = inFlightDelegationByWallet.get(key)

		if (running?.operation === operation) {
			this.logger.debug(
				{ chain, operation },
				"Joining the delegation attempt already in flight for this wallet and chain",
			)
			return running.attempt
		}

		const attempt = running ? running.attempt.catch(() => false).then(work) : work()
		inFlightDelegationByWallet.set(key, { operation, attempt })
		// Clear only our own entry: a later attempt that replaced it must not be dropped.
		const settle = () => {
			if (inFlightDelegationByWallet.get(key)?.attempt === attempt) inFlightDelegationByWallet.delete(key)
		}
		attempt.then(settle, settle)
		return attempt
	}

	private async setupDelegationOnce(chain: string): Promise<boolean> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			this.logger.error("solverAccountContractAddress not configured")
			return false
		}

		if (await this.isDelegated(chain)) {
			unconfirmedDelegationOps.delete(this.walletKey(chain))
			this.logger.info({ chain }, "EOA already delegated to SolverAccount")
			// Delegated does NOT imply bootstrapped. An account delegated by a release that
			// charged EIP-2612 permits has no Permit2 allowance at all, and the delegation op
			// that would have installed one never runs again. Without this, the first sponsored
			// op falls into a native-funded approve — the exact trap the 2026-09-02 `skipPermit`
			// entry in docs/ai/Decisions.md records hitting on Base and Arbitrum.
			await this.ensurePermit2Allowance(chain)
			return true
		}

		// An earlier sponsored op that never confirmed is settled before anything new goes out:
		// it may still be in the bundler's pool, so it is resolved by receipt, EntryPoint nonce or
		// bundler lookup rather than by building a second op on its nonce.
		const earlier = await this.resolveUnconfirmedOp(chain)
		if (earlier === "delegated") return true
		if (earlier === "failed" || earlier === "pending") {
			this.logger.warn({ chain, outcome: earlier }, "An earlier sponsored delegation op has not settled; not sending another")
			return false
		}

		const authority = this.signer.address as HexString

		// The approve is the tx payload, so a token that rejects it (a blacklist, insufficient
		// batched gas) reverts the whole tx with it. The batched attempt is best-effort: a
		// reverted tx usually still delegated (EIP-7702 applies authorization tuples before
		// execution and keeps them applied when execution reverts), and only a genuinely
		// undelegated account carries on to the sponsored path; the approval then defers to
		// the first sponsored op's own funded approve.
		//
		// A permit-capable token skips this: its approve rides the sponsored op instead,
		// which costs the operator stablecoins rather than native.
		const pendingApproval = await this.resolvePendingPermit2Approval(chain)
		if (
			pendingApproval &&
			!pendingApproval.permitCapable &&
			(await this.nativeCoversDirectTx(chain, authority)).covered
		) {
			this.logger.info(
				{ chain, authority, solverAccountContract, token: pendingApproval.token },
				"Setting up EIP-7702 delegation via direct tx with the Permit2 approve batched in",
			)
			if (await this.trySendDelegation(chain, solverAccountContract, pendingApproval)) {
				return true
			}
			if (await this.isDelegated(chain)) {
				this.logger.info(
					{ chain },
					"Batched approve reverted but the delegation landed; approval deferred to the first sponsored op",
				)
				return true
			}
			this.logger.warn({ chain }, "Batched delegate+approve failed; trying the sponsored path")
		}

		if (hasPaymaster(chain, this.configService)) {
			const outcome = await this.setupDelegationViaBundler(chain, pendingApproval)
			if (outcome === "delegated") return true
			if (outcome !== "unavailable") {
				// The op may still land ("pending") or was mined without delegating ("failed").
				// Either way a native tx now could delegate twice, so stop and let the caller
				// retry on the next cycle.
				this.logger.warn({ chain, outcome }, "Not falling back to a direct delegation tx")
				return false
			}
			this.logger.info({ chain }, "Falling back to direct delegation tx")
		}

		// Fallback: direct type-0x04 transaction (requires native token). If the EOA can't
		// cover it, delegation fails outright (the paymaster path already failed too), so
		// surface the deficit explicitly.
		const native = await this.nativeCoversDirectTx(chain, authority)
		if (!native.covered) {
			this.logger.error(
				{
					chain,
					authority,
					nativeBalance: formatEther(native.nativeBalance),
					requiredNative: formatEther(native.requiredNative),
				},
				"Delegation failed: insufficient native balance for direct EIP-7702 tx and paymaster path unavailable",
			)
			return false
		}

		this.logger.info(
			{ chain, authority, solverAccountContract, mode: this.signer.mode ?? "custom" },
			"Setting up EIP-7702 delegation via direct tx",
		)
		// A delegation that landed is success even when the confirmation read failed.
		return (await this.trySendDelegation(chain, solverAccountContract, undefined)) || this.isDelegated(chain)
	}

	/** Whether the EOA can pay for one direct set-code tx at the current gas price. */
	private async nativeCoversDirectTx(
		chain: string,
		authority: HexString,
	): Promise<{ covered: boolean; nativeBalance: bigint; requiredNative: bigint }> {
		const publicClient = this.clientManager.getPublicClient(chain)
		const [nativeBalance, gasPrice] = await Promise.all([
			publicClient.getBalance({ address: authority }),
			publicClient.getGasPrice(),
		])
		const requiredNative = DELEGATION_TX_GAS_FLOOR * gasPrice
		return { covered: nativeBalance >= requiredNative, nativeBalance, requiredNative }
	}

	/**
	 * Sends one native EIP-7702 delegation tx (optionally batching a Permit2 approve) and
	 * waits for its receipt. Never throws: a revert or send error is logged and returns
	 * false, so the caller can fall back. Rebuilds the authorization each call, so a fresh
	 * nonce is used after a prior attempt consumed one.
	 */
	private async trySendDelegation(
		chain: string,
		solverAccountContract: HexString,
		approval: { token: HexString; spender: HexString } | undefined,
	): Promise<boolean> {
		try {
			const { authorization, txNonce } = await this.buildAuthorization(chain, solverAccountContract)
			const hash = await this.sendDelegationTransaction(chain, authorization, txNonce, approval)
			this.logger.info({ chain, txHash: hash, batchedApproval: approval?.token }, "Delegation transaction sent")

			const receipt = await this.clientManager.getPublicClient(chain).waitForTransactionReceipt({ hash })
			if (receipt.status === "success") {
				// A success receipt is not proof the delegation applied — EIP-7702 skips a
				// stale tuple without reverting. The code at the mined block is the proof.
				if (!(await this.confirmDelegationAt(chain, receipt.blockNumber))) {
					this.logger.error(
						{ chain, txHash: hash, blockNumber: receipt.blockNumber },
						"Delegation tx succeeded but the wallet is not delegated — the authorization was skipped",
					)
					return false
				}
				this.logger.info({ chain, txHash: hash, blockNumber: receipt.blockNumber }, "Delegation successful")
				return true
			}
			this.logger.error({ chain, txHash: hash, status: receipt.status }, "Delegation transaction reverted")
			return false
		} catch (error) {
			this.logger.error({ chain, error, batchedApproval: approval?.token }, "Failed to send delegation tx")
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
	 * Revokes delegation by delegating to the zero address, under the same per-wallet/chain
	 * lock setupDelegation uses so a revoke and a setup cannot overlap.
	 */
	async revokeDelegation(chain: string): Promise<boolean> {
		return this.withDelegationLock(chain, "revoke", () => this.revokeDelegationOnce(chain))
	}

	private async revokeDelegationOnce(chain: string): Promise<boolean> {
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info(
				{ chain, authority: this.signer.address, mode: this.signer.mode ?? "custom" },
				"Revoking EIP-7702 delegation",
			)

			const { authorization, txNonce } = await this.buildAuthorization(chain, zeroAddress)
			const hash = await this.sendDelegationTransaction(chain, authorization, txNonce)

			const receipt = await publicClient.waitForTransactionReceipt({ hash })

			if (receipt.status === "success") {
				// A success receipt does not prove the tuple applied — a skipped one is
				// silently dropped. The code must be gone at the mined block.
				if (!(await this.confirmRevokedAt(chain, receipt.blockNumber))) {
					this.logger.error(
						{ chain, txHash: hash, blockNumber: receipt.blockNumber },
						"Revoke tx succeeded but code remains on the wallet — the authorization was skipped",
					)
					return false
				}
				this.logger.info({ chain, txHash: hash }, "Delegation revoked successfully")
				// The revoke consumed the EOA nonce the pending op's authorization was signed
				// for, so that op can no longer delegate. Keeping its record would leave a later
				// setup "pending" on an op that cannot land.
				unconfirmedDelegationOps.delete(this.walletKey(chain))
				return true
			}

			return false
		} catch (error) {
			this.logger.error({ chain, error }, "Failed to revoke delegation")
			return false
		}
	}

	/** Confirms the EOA's code is empty at the block a revoke tx mined in. */
	private async confirmRevokedAt(chain: string, blockNumber: bigint): Promise<boolean> {
		try {
			const code = await this.clientManager.getPublicClient(chain).getCode({
				address: this.signer.address as HexString,
				blockNumber,
			})
			return !code || code === "0x"
		} catch (error) {
			this.logger.error({ chain, blockNumber, error }, "Could not confirm the revoke at the mined block")
			return false
		}
	}
}
