import { privateKeyToAccount } from "viem/accounts"
import { type HexString } from "@hyperbridge/sdk"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"

/** EIP-7702 delegation indicator prefix */
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/**
 * Service for managing EIP-7702 delegation of the filler's EOA to the SolverAccount contract.
 * This enables the filler to participate in solver selection mode.
 */
export class DelegationService {
	private logger = getLogger("delegation-service")

	constructor(
		private clientManager: ChainClientManager,
		private configService: FillerConfigService,
		private privateKey: HexString,
	) {}

	/**
	 * Checks if the filler's EOA is already delegated to the SolverAccount contract on a specific chain.
	 *
	 * @param chain - The chain identifier (e.g., "EVM-1")
	 * @returns True if delegated, false otherwise
	 */
	async isDelegated(chain: string): Promise<boolean> {
		const client = this.clientManager.getPublicClient(chain)
		const account = privateKeyToAccount(this.privateKey)
		const solverAccountContract = this.configService.getSolverAccountContractAddress()

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
		const solverAccountContract = this.configService.getSolverAccountContractAddress()

		if (!solverAccountContract) {
			this.logger.error("solverAccountContractAddress not configured")
			return false
		}

		// Check if already delegated
		if (await this.isDelegated(chain)) {
			this.logger.info({ chain }, "EOA already delegated to SolverAccount")
			return true
		}

		const account = privateKeyToAccount(this.privateKey)
		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info({ chain, eoa: account.address, solverAccountContract }, "Setting up EIP-7702 delegation")

			// Sign the authorization to delegate to SolverAccount
			// viem's experimental EIP-7702 support
			const authorization = await walletClient.signAuthorization({
				account,
				contractAddress: solverAccountContract,
			})

			// Send a transaction with the authorization list to establish delegation
			// The transaction can be to any address (even self) - the important part is the authorizationList
			const hash = await walletClient.sendTransaction({
				account,
				to: account.address,
				value: 0n,
				authorizationList: [authorization],
				chain: walletClient.chain,
			})

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
		const account = privateKeyToAccount(this.privateKey)
		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		try {
			this.logger.info({ chain, eoa: account.address }, "Revoking EIP-7702 delegation")

			// Delegate to zero address to revoke
			const authorization = await walletClient.signAuthorization({
				account,
				contractAddress: "0x0000000000000000000000000000000000000000",
			})

			const hash = await walletClient.sendTransaction({
				account,
				to: account.address,
				value: 0n,
				authorizationList: [authorization],
				chain: walletClient.chain,
			})

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
