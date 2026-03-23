import type { HexString } from "@hyperbridge/sdk"
import { concat, keccak256, toHex, toRlp, zeroAddress } from "viem"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import type { SigningAccount } from "./wallet"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/** Floor for set-code (0x04) txs; L2s like Arbitrum reject viem's default ~94k with "intrinsic gas too low". */
const DELEGATION_TX_GAS_FLOOR = 350_000n

/**
 * Service for managing EIP-7702 delegation of the filler's EOA to the SolverAccount contract.
 * This enables the filler to participate in solver selection mode.
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

	private async buildAuthorization(
		chain: string,
		contractAddress: HexString,
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
			blockTag: "pending",
		})
		// EIP-7702 auth nonce must be authority tx nonce + 1 when authority submits the type-0x04 tx.
		const authorizationNonce = currentNonce + 1
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
	 *
	 * @param chain - The chain identifier (e.g., "EVM-1")
	 * @returns True if delegated, false otherwise
	 */
	async isDelegated(chain: string): Promise<boolean> {
		const client = this.clientManager.getPublicClient(chain)
		const account = this.signer.account
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			return false
		}

		try {
			// Get the code at the filler's EOA address
			const code = await client.getCode({ address: account.address })

			if (!code || code === "0x") {
				return false
			}

			// Check if code starts with delegation indicator (0xef0100 + address)
			// EIP-7702 sets code to: 0xef0100 || delegate_address (23 bytes total)
			if (code.toLowerCase().startsWith(DELEGATION_INDICATOR_PREFIX)) {
				const delegatedTo = ("0x" + code.slice(8)) as HexString // Skip "0xef0100"
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
	 * Sets up EIP-7702 delegation from the filler's EOA to the SolverAccount contract.
	 * This is required for solver selection mode to work.
	 *
	 * @param chain - The chain identifier to set up delegation on
	 * @returns True if delegation was successful or already in place
	 */
	async setupDelegation(chain: string): Promise<boolean> {
		const solverAccountContract = this.configService.getSolverAccountContractAddress(chain)

		if (!solverAccountContract) {
			this.logger.error("solverAccountContractAddress not configured")
			return false
		}

		// Check if already delegated
		if (await this.isDelegated(chain)) {
			this.logger.info({ chain }, "EOA already delegated to SolverAccount")
			return true
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const authority = this.signer.account

		try {
			this.logger.info(
				{
					chain,
					authority: authority.address,
					solverAccountContract,
					mode: this.signer.mode,
				},
				"Setting up EIP-7702 delegation",
			)

			const authorization = await this.buildAuthorization(chain, solverAccountContract)
			const hash = await this.sendDelegationTransaction(chain, authorization)

			this.logger.info({ chain, txHash: hash }, "Delegation transaction sent")

			// Wait for confirmation
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
	 *
	 * @param chains - Array of chain identifiers (e.g., ["EVM-1", "EVM-137"])
	 * @returns Object with results per chain
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
	 * This restores the EOA to a normal (non-delegated) state.
	 *
	 * @param chain - The chain identifier
	 * @returns True if revocation was successful
	 */
	async revokeDelegation(chain: string): Promise<boolean> {
		const authority = this.signer.account
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info(
				{ chain, authority: authority.address, mode: this.signer.mode },
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
