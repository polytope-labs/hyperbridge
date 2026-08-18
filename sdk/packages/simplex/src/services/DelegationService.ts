import type { HexString } from "@hyperbridge/sdk"
import { formatEther, zeroAddress } from "viem"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { type Logger , moduleLogger} from "./Logger"
import type { Signer } from "./wallet"
import { hasPaymaster } from "./paymaster"
import { UserOpSender } from "./UserOpSender"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/**
 * Fixed gas limit for set-code (0x04) txs.
 */
const DELEGATION_TX_GAS_FLOOR = 650_000n

/**
 * Service for managing EIP-7702 delegation of the filler's EOA to the SolverAccount contract.
 * This enables the filler to participate in solver selection mode.
 *
 * When a paymaster (Circle or Simplex) is configured and the filler holds stablecoins,
 * delegation is performed via a no-op UserOp sent through the bundler — the paymaster
 * pays gas in stablecoins, so the solver never needs native tokens. Falls back to a
 * direct type-0x04 tx if the bundler path is unavailable.
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
	): Promise<HexString> {
		const walletClient = this.clientManager.getWalletClient(chain)

		// Every backend goes out the same way: viem prepares the set-code tx and the
		// signer signs it. A backend whose transaction API cannot express an
		// authorization list handles that in its own `signTransaction` — see
		// `mpcVaultSigner` — not here.
		return (await walletClient.sendTransaction({
			to: this.signer.address,
			value: 0n,
			authorizationList: [authorization],
			chain: walletClient.chain,
			gas: DELEGATION_TX_GAS_FLOOR,
		})) as HexString
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
	 * Sets up EIP-7702 delegation via the bundler with a no-op UserOp.
	 * Uses the Circle Paymaster (USDC permit) when available and filler has USDC balance.
	 */
	private async setupDelegationViaBundler(chain: string): Promise<boolean> {
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
			// first-time cold storage, so the proven 150k account + default 200k paymaster
			// limits clear rundler's verification-efficiency policy. A RE-delegation
			// (EOA already delegated) uses much less (warm slots, paymaster allowance
			// reused) — actual ~96k — so those loose limits fall below the 0.4 floor
			// (`actual / (accountVerif + paymasterVerif)`). Tighten both verification
			// limits for that case so the ratio clears 0.4 while still covering usage.
			//
			// The tightened paymaster limit assumes the allowance is still in place. When
			// it has been depleted (or was never set on this chain — e.g. re-delegating
			// from an older SolverAccount), the Circle builder ignores the override and
			// keeps its 200k default: the permit executed during validation needs ~113k
			// on its own and would OOG the paymaster frame (bundler AA33) at 110k.
			const code = await this.clientManager.getPublicClient(chain).getCode({
				address: this.signer.address as HexString,
			})
			const isFreshEoa = !code || code === "0x"

			// The EIP-7702 authorization rides inside the UserOp so a not-yet-delegated
			// EOA is delegated in the op (bundler submits the tx, so it uses the current
			// nonce). Passed as a factory: the sender signs it only after paymaster data
			// is built, because approve-mode chains send an approve tx from this same EOA
			// at that step, which would invalidate an authorization signed beforehand.
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData: "0x" as HexString,
				eip7702Auth: () => this.buildAuthorization(chain, solverAccountContract, true),
				gas: isFreshEoa
					? { verificationGasLimit: 150_000n, callGasLimit: 50_000n, preVerificationGas: 100_000n }
					: { verificationGasLimit: 80_000n, callGasLimit: 50_000n, preVerificationGas: 100_000n },
				paymasterVerificationGasLimit: isFreshEoa ? undefined : 110_000n,
				forceApproveMode: true,
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
		const authority = this.signer.address as HexString

		// The direct tx pays gas in native ETH. If the EOA can't cover it, delegation fails
		// outright (the paymaster path already failed too) — surface the deficit explicitly.
		const [nativeBalance, gasPrice] = await Promise.all([
			publicClient.getBalance({ address: authority }),
			publicClient.getGasPrice(),
		])
		const requiredNative = DELEGATION_TX_GAS_FLOOR * gasPrice
		if (nativeBalance < requiredNative) {
			this.logger.error(
				{
					chain,
					authority,
					nativeBalance: formatEther(nativeBalance),
					requiredNative: formatEther(requiredNative),
				},
				"Delegation failed: insufficient native balance for direct EIP-7702 tx and paymaster path unavailable",
			)
			return false
		}

		try {
			this.logger.info(
				{ chain, authority, solverAccountContract, mode: this.signer.mode ?? "custom" },
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
