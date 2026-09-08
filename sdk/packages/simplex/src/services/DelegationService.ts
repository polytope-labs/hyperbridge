import type { HexString } from "@hyperbridge/sdk"
import { encodeFunctionData, erc20Abi, formatEther, maxUint256, zeroAddress } from "viem"
import { encodeERC7821ExecuteBatch } from "@hyperbridge/sdk"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { type Logger, moduleLogger } from "./Logger"
import type { Signer } from "./wallet"
import { hasPaymaster } from "./paymaster"
import { resolvePendingPermit2Approval } from "./paymaster/provider/simplex"
import { UserOpSender } from "./UserOpSender"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/**
 * Fixed gas limit for set-code (0x04) txs.
 */
const DELEGATION_TX_GAS_FLOOR = 650_000n

/** Extra gas for an ERC-20 approve folded into the delegation tx. */
const BATCHED_APPROVE_GAS = 60_000n

/** callGasLimit for a delegation op carrying `approve(Permit2, max)` through ERC-7821. */
const BOOTSTRAP_CALL_GAS_LIMIT = 150_000n

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
		const authorityAddress = this.signer.address as HexString
		const currentNonce = await publicClient.getTransactionCount({
			address: authorityAddress,
			blockTag: "latest",
		})
		const authorizationNonce = viaBundler ? currentNonce : currentNonce + 1

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
			})) as HexString
		}

		return (await walletClient.sendTransaction({
			to: this.signer.address,
			value: 0n,
			authorizationList: [authorization],
			chain: walletClient.chain,
			gas: DELEGATION_TX_GAS_FLOOR,
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
	 */
	async isDelegated(chain: string): Promise<boolean> {
		const client = this.clientManager.getPublicClient(chain)
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			return false
		}

		try {
			const code = await client.getCode({ address: this.signer.address })

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
	 * Sets up EIP-7702 delegation via the bundler. Uses the Simplex paymaster when
	 * configured and the filler holds a sufficient stablecoin balance.
	 *
	 * The op is a no-op once the account has a Permit2 allowance. On a chain it has
	 * never used, and only when the fee token implements EIP-2612 (`permitCapable`),
	 * the op instead carries `approve(Permit2, max)` as its callData and asks the
	 * paymaster for PERMIT mode: the permit covers this op's gas, and the op installs
	 * the allowance every later op authorizes against. That is the whole zero-native
	 * bootstrap — one sponsored op, after which the account never needs a permit again.
	 */
	private async setupDelegationViaBundler(
		chain: string,
		pendingApproval?: { token: HexString; spender: HexString; permitCapable: boolean } | null,
	): Promise<boolean> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract || !this.userOpSender.canSponsor(chain)) {
			this.logger.warn({ chain }, "Missing config for bundler-based delegation, falling back to direct tx")
			return false
		}

		try {
			this.logger.info(
				{ chain, solverAccount: this.signer.address, solverAccountContract, mode: "bundler" },
				"Setting up EIP-7702 delegation via bundler with paymaster",
			)

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
			// (VERIFICATION_GAS_LIMIT_PERMIT2, 200k), charged to the paymaster frame.
			const code = await this.clientManager.getPublicClient(chain).getCode({
				address: this.signer.address as HexString,
			})
			const isFreshEoa = !code || code === "0x"

			// Bootstrap only when the token can pay for it: without a permit the paymaster
			// would have to land a native-funded approve first, and that same approve would
			// then make this op's callData a non-zero → non-zero change the USDT rule
			// rejects. Those chains stay on the no-op op and the native fallbacks.
			const bootstrap = !!pendingApproval?.permitCapable
			const callData = bootstrap
				? encodeERC7821ExecuteBatch([
						{
							target: pendingApproval!.token,
							value: 0n,
							data: encodeFunctionData({
								abi: erc20Abi,
								functionName: "approve",
								args: [pendingApproval!.spender, maxUint256],
							}),
						},
					])
				: ("0x" as HexString)

			// The EIP-7702 authorization rides inside the UserOp so a not-yet-delegated
			// EOA is delegated in the op (bundler submits the tx, so it uses the current
			// nonce). Passed as a factory: the sender signs it only after paymaster data
			// is built, because a first-time approval (to Permit2 or the paymaster) sends
			// a tx from this same EOA at that step, invalidating an earlier authorization.
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData,
				eip7702Auth: () => this.buildAuthorization(chain, solverAccountContract, true),
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
				this.logger.info(
					{ chain, txHash: result.txHash },
					"Delegation via bundler successful — paymaster paid gas",
				)
				return true
			}

			// null → the op was never submitted (no paymaster/bundler, insufficient USDC,
			// or the bundler rejected it). Safe to fall back to a direct tx.
			this.logger.warn({ chain }, "Sponsored delegation unavailable, falling back to direct tx")
			return false
		} catch (error) {
			// The op may have been submitted but not yet confirmed — check on-chain status
			// rather than blindly re-submitting via a direct tx.
			this.logger.warn({ chain, error }, "Bundler delegation did not confirm, checking on-chain status")
			return this.isDelegated(chain)
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
			const success = await this.setupDelegationViaBundler(chain, pendingApproval)
			if (success) return true
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
		return this.trySendDelegation(chain, solverAccountContract, undefined)
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
			const authorization = await this.buildAuthorization(chain, solverAccountContract)
			const hash = await this.sendDelegationTransaction(chain, authorization, approval)
			this.logger.info({ chain, txHash: hash, batchedApproval: approval?.token }, "Delegation transaction sent")

			const receipt = await this.clientManager.getPublicClient(chain).waitForTransactionReceipt({ hash })
			if (receipt.status === "success") {
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
	 * Revokes delegation by delegating to the zero address.
	 */
	async revokeDelegation(chain: string): Promise<boolean> {
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info(
				{ chain, authority: this.signer.address, mode: this.signer.mode ?? "custom" },
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
}
