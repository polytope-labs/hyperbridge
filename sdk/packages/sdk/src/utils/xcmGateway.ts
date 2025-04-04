import type { ApiPromise } from "@polkadot/api"
import type { HexString } from "@/types"
import type { SignerOptions } from "@polkadot/api/types"
import { u8aToHex } from "@polkadot/util"
import { decodeAddress } from "@polkadot/util-crypto"
import { parseUnits } from "viem"
import type { IndexerClient } from "@/client"
import { sleep } from "@/utils"

export type HyperbridgeTxEvents =
	| {
			kind: "Ready"
			transaction_hash: HexString
	  }
	| {
			kind: "Dispatched"
			transaction_hash: HexString
			block_number: bigint
			commitment: HexString
	  }
	| {
			kind: "Finalized"
			transaction_hash: HexString
			block_number?: bigint
			commitment?: HexString
	  }
	| {
			kind: "Error"
			error: unknown
	  }

const DECIMALS = 10
/**
 * Parameters for teleporting DOT from Polkadot relay chain to EVM-based destination
 */
export type XcmGatewayParams = {
	/**
	 * Destination state machine ID (chain ID) where assets will be teleported to
	 * This value identifies the specific EVM chain in the destination network
	 */
	destination: number

	/**
	 * The recipient address on the destination chain (in hex format)
	 * This is the EVM address that will receive the teleported assets
	 */
	recipient: HexString

	/**
	 * Amount of DOT to teleport
	 * This will be converted to the appropriate format internally
	 */
	amount: number

	/**
	 * Request timeout value in blocks or timestamp
	 * Specifies how long the teleport request remains valid before expiring
	 */
	timeout: bigint

	/**
	 * The parachain ID of the Hyperbridge
	 */
	paraId: number
}

/**
 * Teleports DOT tokens from Polkadot relay chain to an EVM-based destination chain
 * using XCM (Cross-Consensus Message Format) and uses the indexer client to track
 * the transaction instead of polling hyperbridge blocks.
 *
 * This function initiates a teleport transaction, monitors its status through the indexer,
 * and yields events about the transaction's progress through a ReadableStream.
 * It handles the complete lifecycle of a teleport operation:
 * 1. Transaction preparation and signing
 * 2. Broadcasting to the relay chain
 * 3. Tracking the transaction via the indexer client
 * 4. Yielding events about transaction status
 *
 * Note: There is no guarantee that both Dispatched and Finalized events will be yielded.
 * Consumers should listen for either one of these events instead of expecting both.
 *
 * @param relayApi - Polkadot API instance connected to the relay chain
 * @param hyperbridge - Polkadot API instance connected to the Hyperbridge parachain
 * @param who - Sender's SS58Address address
 * @param options - Transaction signing options
 * @param params - Teleport parameters including destination, recipient, and amount
 * @param indexerClient - The indexer client to track the transaction
 * @param pollInterval - Optional polling interval in milliseconds (default: 2000)
 * @yields {HyperbridgeTxEvents} Stream of events indicating transaction status
 */
export async function teleportDot(param_: {
	relayApi: ApiPromise
	hyperbridge: ApiPromise
	who: string
	xcmGatewayParams: XcmGatewayParams
	indexerClient: IndexerClient
	pollInterval?: number
	options: Partial<SignerOptions>
}): Promise<ReadableStream<HyperbridgeTxEvents>> {
	const { relayApi, hyperbridge, who, options, xcmGatewayParams: params, indexerClient, pollInterval = 2000 } = param_

	// Set up the transaction parameters
	const destination = {
		V3: {
			parents: 0,
			interior: {
				X1: {
					Parachain: params.paraId,
				},
			},
		},
	}

	const beneficiary = {
		V3: {
			parents: 0,
			interior: {
				X3: [
					{
						AccountId32: {
							id: u8aToHex(decodeAddress(who)),
							network: null,
						},
					},
					{
						AccountKey20: {
							network: {
								Ethereum: {
									chainId: params.destination,
								},
							},
							key: params.recipient,
						},
					},
					{
						GeneralIndex: params.timeout,
					},
				],
			},
		},
	}
	const assets = {
		V3: [
			{
				id: {
					Concrete: {
						parents: 0,
						interior: "Here",
					},
				},
				fun: {
					Fungible: parseUnits(params.amount.toString(), DECIMALS),
				},
			},
		],
	}

	const feeAssetItem = 0
	const weightLimit = "Unlimited"

	const tx = relayApi.tx.xcmPallet.limitedReserveTransferAssets(
		destination,
		beneficiary,
		assets,
		feeAssetItem,
		weightLimit,
	)

	const finalized_hash = await hyperbridge.rpc.chain.getFinalizedHead()
	const hyperbridgeBlock = (await hyperbridge.rpc.chain.getHeader(finalized_hash)).number.toNumber()

	let closed = false
	// Create the stream to report events
	let unsubscribe: () => void
	const stream = new ReadableStream<HyperbridgeTxEvents>(
		{
			async start(controller) {
				unsubscribe = await tx.signAndSend(who, options, async (result) => {
					try {
						const { status, dispatchError, txHash } = result

						if (dispatchError) {
							controller.enqueue({
								kind: "Error",
								error: `Error watching extrinsic: ${dispatchError.toString()}`,
							})
							unsubscribe?.()
							controller.close()
							closed = true
							return
						}

						if (status.isReady) {
							// Send tx hash as soon as it is available
							controller.enqueue({
								kind: "Ready",
								transaction_hash: txHash.toHex(),
							})
						} else if (status.isInBlock || status.isFinalized) {
							// Get the sender address in hex format for the indexer query
							const decodedWho = u8aToHex(decodeAddress(who, false))

							// Poll the indexer until we find the AssetTeleported event
							let assetTeleported = undefined
							let attempts = 0
							// Calculate max attempts for 5 minutes of polling (300 seconds)
							const maxAttempts = Math.ceil(300000 / pollInterval) // 5 minutes in milliseconds / poll interval

							// If indexerClient is not defined, throw an error
							if (!indexerClient) {
								controller.enqueue({
									kind: "Error",
									error: "IndexerClient is required but not provided",
								})
								return
							}

							while (!assetTeleported && attempts < maxAttempts) {
								await sleep(pollInterval)

								assetTeleported = await indexerClient.queryAssetTeleported(
									decodedWho,
									params.recipient.toLowerCase(),
									params.destination.toString(),
									hyperbridgeBlock,
								)

								attempts++
							}

							if (!assetTeleported) {
								controller.enqueue({
									kind: "Error",
									error: "Failed to locate AssetTeleported event in the indexer after maximum attempts",
								})
								return
							}

							// We found the asset teleported event through the indexer
							const commitment = assetTeleported.commitment as HexString
							const blockNumber = BigInt(assetTeleported.blockNumber)

							// Send event with the status kind (either Dispatched or Finalized)
							controller.enqueue({
								kind: "Finalized",
								transaction_hash: txHash.toHex(),
								block_number: blockNumber,
								commitment: commitment,
							})

							// We can end the stream because indexer only indexes finalized events from hyperbridge
							closed = true
							unsubscribe?.()
							controller.close()
							return
						}
					} catch (err) {
						// For some unknown reason the call back is called again after unsubscribing, this check prevents it from trying to push an event to the closed stream
						if (closed) {
							return
						}
						controller.enqueue({
							kind: "Error",
							error: String(err),
						})
					}
				})
			},
			cancel() {
				// This is called if the reader cancels,
				unsubscribe?.()
			},
		},
		{
			highWaterMark: 3,
			size: () => 1,
		},
	)

	return stream
}
