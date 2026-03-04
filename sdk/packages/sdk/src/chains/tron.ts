import type { PublicClient, TransactionReceipt } from "viem"
import { TronWeb } from "tronweb"

import type { IChain, IIsmpMessage } from "@/chain"
import { EvmChain, type EvmChainParams } from "@/chains/evm"
import type { HexString, IGetRequest, IMessage, IPostRequest, StateMachineHeight, StateMachineIdParams } from "@/types"
import { retryPromise } from "@/utils"

/**
 * Parameters for a Tron-backed chain.
 * TronWeb is constructed internally from `rpcUrl`.
 */
export type TronChainParams = EvmChainParams

export class TronChain implements IChain {
	private readonly evm: EvmChain
	private readonly tronWebInstance: InstanceType<typeof TronWeb>

	constructor(private readonly params: TronChainParams) {
		this.evm = new EvmChain(params)
		this.tronWebInstance = new TronWeb({ fullHost: params.rpcUrl })
	}

	// -------------------------------------------------------------------------
	// Public accessors
	// -------------------------------------------------------------------------

	/** Underlying viem public client (delegated from the internal EvmChain) */
	get client(): PublicClient {
		return this.evm.client
	}

	/** Host contract address for this chain (delegated from the internal EvmChain) */
	get host(): HexString {
		return this.evm.host
	}

	/** Chain configuration (delegated from the internal EvmChain) */
	get config() {
		return this.evm.config
	}

	/** Chain configuration service (delegated from the internal EvmChain) */
	get configService() {
		return this.evm.configService
	}

	/** TronWeb instance for this Tron chain (constructed from rpcUrl) */
	get tronWeb(): InstanceType<typeof TronWeb> {
		return this.tronWebInstance
	}

	// -------------------------------------------------------------------------
	// IChain implementation (delegated to the internal EvmChain)
	// -------------------------------------------------------------------------

	timestamp(): Promise<bigint> {
		return this.evm.timestamp()
	}

	requestReceiptKey(commitment: HexString): HexString {
		return this.evm.requestReceiptKey(commitment)
	}

	queryRequestReceipt(commitment: HexString): Promise<HexString | undefined> {
		return this.evm.queryRequestReceipt(commitment)
	}

	queryStateProof(at: bigint, keys: HexString[], address?: HexString): Promise<HexString> {
		return this.evm.queryStateProof(at, keys, address)
	}

	queryProof(message: IMessage, counterparty: string, at?: bigint): Promise<HexString> {
		return this.evm.queryProof(message, counterparty, at)
	}

	encode(message: IIsmpMessage): HexString {
		return this.evm.encode(message)
	}

	latestStateMachineHeight(stateMachineId: StateMachineIdParams): Promise<bigint> {
		return this.evm.latestStateMachineHeight(stateMachineId)
	}

	challengePeriod(stateMachineId: StateMachineIdParams): Promise<bigint> {
		return this.evm.challengePeriod(stateMachineId)
	}

	stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint> {
		return this.evm.stateMachineUpdateTime(stateMachineHeight)
	}

	/**
	 * Retrieves top-level calldata from a Tron transaction via TronWeb.
	 * Only works for direct calls to IntentGateway (not nested/multicall).
	 * Tron does not support debug_traceTransaction.
	 */
	async getPlaceOrderCalldata(txHash: string, _intentGatewayAddress: string): Promise<HexString> {
		const tx = await retryPromise(() => this.tronWeb.trx.getTransaction(txHash), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: `Failed to get Tron transaction ${txHash}`,
		})

		const rawData = (tx?.raw_data?.contract?.[0]?.parameter?.value as any)?.data
		if (!rawData) {
			throw new Error(`No calldata found in Tron transaction ${txHash}`)
		}

		return (rawData.startsWith("0x") ? rawData : `0x${rawData}`) as HexString
	}

	/**
	 * Broadcasts a signed Tron transaction and waits for confirmation,
	 * returning a 0x-prefixed transaction hash compatible with viem.
	 *
	 * This mirrors the behavior used in IntentGatewayV2 for Tron chains.
	 */
	async broadcastTransaction(signedTransaction: any): Promise<TransactionReceipt> {
		const tronReceipt = await this.tronWeb.trx.sendRawTransaction(signedTransaction)
		if (!tronReceipt.result) {
			throw new Error("Tron transaction broadcast failed")
		}

		const tronTxId = tronReceipt.transaction.txID
		const receipt = await this.client.waitForTransactionReceipt({
			hash: `0x${tronTxId}`,
			confirmations: 1,
		})

		if (!receipt) {
			throw new Error("Tron transaction receipt not found")
		}

		return receipt
	}

	// -------------------------------------------------------------------------
	// Helpers mirrored from EvmChain for protocol integrations
	// -------------------------------------------------------------------------

	/** Gets the fee token address and decimals for the chain. */
	getFeeTokenWithDecimals(): Promise<{ address: HexString; decimals: number }> {
		return this.evm.getFeeTokenWithDecimals()
	}

	/** Gets the nonce of the host contract on this chain. */
	getHostNonce(): Promise<bigint> {
		return this.evm.getHostNonce()
	}

	/** Quotes the fee (in native token) required for the given ISMP request. */
	quoteNative(request: IPostRequest | IGetRequest, fee: bigint): Promise<bigint> {
		return this.evm.quoteNative(request, fee)
	}

	/** Estimates gas for executing a post request on this chain. */
	estimateGas(request: IPostRequest): Promise<{ gas: bigint; postRequestCalldata: HexString }> {
		return this.evm.estimateGas(request)
	}
}
