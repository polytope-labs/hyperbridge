import { BridgeKit, isKitError, isRetryableError, getErrorMessage } from "@circle-fin/bridge-kit"
import type { BridgeResult, BridgeParams, EstimateResult } from "@circle-fin/bridge-kit"
import { createViemAdapterFromPrivateKey } from "@circle-fin/adapter-viem-v2"
import type { Chain, PublicClient } from "viem"
import { type HexString, parseStateMachineId } from "@hyperbridge/sdk"
import { ChainClientManager } from "../ChainClientManager"
import { FillerConfigService } from "../FillerConfigService"
import { getLogger, type Logger } from "../Logger"
import { RebalanceOptions } from "."

/** Viem adapter type */
type ViemAdapter = ReturnType<typeof createViemAdapterFromPrivateKey>

/**
 * Maps EVM chain IDs to CCTP chain names used by Bridge Kit
 */
const CHAIN_ID_TO_CCTP: Record<number, string> = {
	1: "Ethereum",
	11155111: "Ethereum_Sepolia",
	42161: "Arbitrum",
	421614: "Arbitrum_Sepolia",
	8453: "Base",
	84532: "Base_Sepolia",
	10: "Optimism",
	11155420: "Optimism_Sepolia",
	137: "Polygon",
	80002: "Polygon_Amoy_Testnet",
	130: "Unichain",
	1301: "Unichain_Sepolia",
}

/**
 * Converts state machine ID to CCTP chain name
 */
function stateMachineToCctpChain(stateMachineId: string): string {
	const chainId = parseStateMachineId(stateMachineId).stateId.Evm
	if (chainId === undefined) {
		throw new Error(`Chain ${stateMachineId} is not an EVM chain`)
	}
	const cctpChain = CHAIN_ID_TO_CCTP[chainId]
	if (!cctpChain) {
		throw new Error(`Chain ${stateMachineId} (chainId: ${chainId}) is not supported by CCTP`)
	}
	return cctpChain
}

/**
 * CctpRebalancer - Cross-chain USDC transfers using Circle's CCTP via BridgeKit
 */
export class CctpRebalancer {
	private bridgeKit: BridgeKit
	private adapter: ViemAdapter | null = null
	private chainClientManager: ChainClientManager
	private privateKey: HexString
	private logger: Logger

	constructor(chainClientManager: ChainClientManager, configService: FillerConfigService, privateKey: HexString) {
		this.chainClientManager = chainClientManager
		this.privateKey = privateKey
		this.bridgeKit = new BridgeKit()
		this.logger = getLogger("CctpRebalancer")
	}

	/**
	 * Creates the viem adapter lazily, using existing public clients from ChainClientManager
	 */
	private getAdapter(): ViemAdapter {
		if (this.adapter) return this.adapter

		this.adapter = createViemAdapterFromPrivateKey({
			privateKey: this.privateKey,
			getPublicClient: ({ chain }: { chain: Chain }) => {
				// Use existing public client from ChainClientManager
				const stateMachineId = `EVM-${chain.id}`
				return this.chainClientManager.getPublicClient(stateMachineId) as PublicClient
			},
		})

		return this.adapter
	}

	/**
	 * Sends USDC cross-chain using CCTP
	 */
	async sendCctp(options: RebalanceOptions): Promise<BridgeResult> {
		const { amount, source, destination, recipientAddress } = options

		const sourceChain = stateMachineToCctpChain(source)
		const destChain = stateMachineToCctpChain(destination)

		this.logger.info({ amount, source: sourceChain, destination: destChain }, "Initiating CCTP transfer")

		const adapter = this.getAdapter()

		const bridgeParams = {
			from: { adapter, chain: sourceChain },
			to: recipientAddress ? { adapter, chain: destChain, recipientAddress } : { adapter, chain: destChain },
			amount,
		} as BridgeParams

		try {
			const result = await this.bridgeKit.bridge(bridgeParams)

			if (result.state === "success") {
				this.logger.info({ amount, source: sourceChain, destination: destChain }, "CCTP transfer completed")
			} else {
				this.logger.error({ state: result.state, steps: result.steps }, "CCTP transfer failed")
			}

			return result
		} catch (error) {
			this.logger.error({ error: getErrorMessage(error) }, "CCTP transfer error")

			if (isKitError(error) && isRetryableError(error)) {
				this.logger.info("Error is retryable")
			}

			throw error
		}
	}

	/**
	 * Estimates the cost of a CCTP transfer
	 */
	async estimateCctp(options: RebalanceOptions): Promise<EstimateResult> {
		const { amount, source, destination, recipientAddress } = options

		const sourceChain = stateMachineToCctpChain(source)
		const destChain = stateMachineToCctpChain(destination)

		const adapter = this.getAdapter()

		const bridgeParams = {
			from: { adapter, chain: sourceChain },
			to: recipientAddress ? { adapter, chain: destChain, recipientAddress } : { adapter, chain: destChain },
			amount,
		} as BridgeParams

		return this.bridgeKit.estimate(bridgeParams)
	}

	/**
	 * Retries a failed CCTP transfer
	 */
	async retrySendCctp(failedResult: BridgeResult): Promise<BridgeResult> {
		if (failedResult.state !== "error") {
			throw new Error("Can only retry failed transfers")
		}

		this.logger.info(
			{
				source: failedResult.source.chain.chain,
				destination: failedResult.destination.chain.chain,
				amount: failedResult.amount,
			},
			"Retrying failed CCTP transfer",
		)

		const adapter = this.getAdapter()

		try {
			const result = await this.bridgeKit.retry(failedResult, {
				from: adapter,
				to: adapter,
			})

			if (result.state === "success") {
				this.logger.info("CCTP retry completed successfully")
			} else {
				this.logger.error({ state: result.state, steps: result.steps }, "CCTP retry failed")
			}

			return result
		} catch (error) {
			this.logger.error({ error: getErrorMessage(error) }, "CCTP retry error")
			throw error
		}
	}
}
