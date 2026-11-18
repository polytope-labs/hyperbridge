import { BridgeKit, isKitError, isRetryableError, getErrorMessage } from "@circle-fin/bridge-kit"
import type { BridgeResult, BridgeParams, EstimateResult } from "@circle-fin/bridge-kit"
import { ViemAdapter } from "@circle-fin/adapter-viem-v2"
import {
	Arbitrum,
	ArbitrumSepolia,
	Base,
	BaseSepolia,
	Ethereum,
	EthereumSepolia,
	Optimism,
	OptimismSepolia,
	Polygon,
	PolygonAmoy,
	Unichain,
	UnichainSepolia,
} from "@circle-fin/bridge-kit/chains"
import type { Chain, PublicClient, WalletClient } from "viem"
import { parseStateMachineId } from "@hyperbridge/sdk"
import type { HexString } from "@hyperbridge/sdk"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { type Logger , moduleLogger} from "@/services/Logger"
import type { RebalanceOptions, RebalanceRoute, RouteTransfer } from "."

/** Viem adapter type */
type ViemAdapterInstance = ViemAdapter

/**
 * The chains this route bridges USDC between, as Bridge Kit defines them. The one list: the
 * adapter is built from it, and so is what the route says it supports.
 */
const CCTP_CHAINS = [
	Ethereum,
	EthereumSepolia,
	Arbitrum,
	ArbitrumSepolia,
	Base,
	BaseSepolia,
	Optimism,
	OptimismSepolia,
	Polygon,
	PolygonAmoy,
	Unichain,
	UnichainSepolia,
]

/** The Bridge Kit chain for a state machine id, or undefined where CCTP does not run. */
function cctpChain(stateMachineId: string) {
	const chainId = parseStateMachineId(stateMachineId).stateId.Evm
	return CCTP_CHAINS.find((chain) => chain.chainId === chainId)
}

/**
 * Converts state machine ID to CCTP chain name
 */
function stateMachineToCctpChain(stateMachineId: string): string {
	const chain = cctpChain(stateMachineId)
	if (!chain) {
		throw new Error(`Chain ${stateMachineId} is not supported by CCTP`)
	}
	return chain.chain
}

/**
 * CctpRebalancer - Cross-chain USDC transfers using Circle's CCTP via BridgeKit
 */
export class CctpRebalancer implements RebalanceRoute {
	readonly method = "cctp"
	readonly assets = ["USDC"]
	/** Bridge Kit mints on the destination from the same wallet that burned on the source. */
	readonly sendsOnDestination = true
	private bridgeKit: BridgeKit
	private adapter: ViemAdapterInstance | null = null
	private chainClientManager: ChainClientManager
	private logger: Logger

	constructor(
		chainClientManager: ChainClientManager,
		_configService: FillerConfigService,
	) {
		this.chainClientManager = chainClientManager
		this.bridgeKit = new BridgeKit()
		this.logger = moduleLogger(chainClientManager.loggers, "CctpRebalancer")
	}

	/**
	 * Creates the viem adapter lazily, using existing public clients from ChainClientManager
	 */
	private getAdapter(): ViemAdapterInstance {
		if (this.adapter) return this.adapter

		this.adapter = new ViemAdapter(
			{
			getPublicClient: ({ chain }: { chain: Chain }) => {
				// Use existing public client from ChainClientManager
				const stateMachineId = `EVM-${chain.id}`
				return this.chainClientManager.getPublicClient(stateMachineId) as PublicClient
			},
				getWalletClient: ({ chain }: { chain: Chain }) => {
					const stateMachineId = `EVM-${chain.id}`
					return this.chainClientManager.getWalletClient(stateMachineId) as WalletClient
				},
			},
			{
				addressContext: "user-controlled",
				supportedChains: CCTP_CHAINS,
			},
		)

		return this.adapter
	}

	/** Circle's USDC on `chain`, which is the only token CCTP moves. */
	tokenOn(asset: string, chain: string): HexString | undefined {
		if (!this.assets.includes(asset)) return undefined
		return cctpChain(chain)?.usdcAddress as HexString | undefined
	}

	/** Burns on the source and mints on the destination. A transfer Bridge Kit did not finish is an error. */
	async send({ amount, source, destination }: RouteTransfer): Promise<{ txHash?: string }> {
		const result = await this.sendCctp({ amount, source, destination })
		// By name: the first step with a hash is the approval, which moves nothing.
		const burn = result.steps.find((step) => step.name === "burn")
		if (result.state !== "success") {
			const failed = result.steps.find((step) => step.state === "error")
			throw new Error(
				`CCTP transfer ended ${result.state}` +
					(failed ? ` at ${failed.name}${failed.errorMessage ? `: ${failed.errorMessage}` : ""}` : "") +
					(burn?.txHash ? ` (source tx ${burn.txHash})` : ""),
			)
		}
		return { txHash: burn?.txHash }
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
