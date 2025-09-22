import type { ApiPromise } from "@polkadot/api"
import type { HexString } from "@/types"
import type { SignerOptions } from "@polkadot/api/types"
import { u8aToHex, hexToU8a } from "@polkadot/util"
import { decodeAddress, keccakAsHex } from "@polkadot/util-crypto"
import { parseUnits } from "viem"
import { sleep, StateMachine } from "@/utils"
import { Bytes, Struct, Tuple, u128, u64, u8 } from "scale-ts"

const MultiAccount = Struct({
	substrate_account: Bytes(32),
	evm_account: Bytes(20),
	dest_state_machine: StateMachine,
	timeout: u64,
	account_nonce: u64,
})

export type HyperbridgeTxEvents =
	| {
			kind: "Ready"
			transaction_hash: HexString
			message_id?: HexString
	  }
	| {
			kind: "Dispatched"
			transaction_hash: HexString
			block_number: bigint
			message_id?: HexString
			commitment: HexString
	  }
	| {
			kind: "Finalized"
			transaction_hash: HexString
			message_id?: HexString
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
	sourceApi: ApiPromise
	sourceIsAssetHub: boolean
	who: string
	xcmGatewayParams: XcmGatewayParams
	options: Partial<SignerOptions>
}): Promise<ReadableStream<HyperbridgeTxEvents>> {
	const { sourceApi, sourceIsAssetHub, who, options, xcmGatewayParams: params } = param_
	let { nonce: accountNonce } = (await sourceApi.query.system.account(who)) as any

	let encoded_message = MultiAccount.enc({
		substrate_account: decodeAddress(who),
		evm_account: hexToU8a(params.recipient),
		dest_state_machine: { tag: "Evm", value: params.destination },
		timeout: params.timeout,
		account_nonce: accountNonce,
	})

	let message_id = keccakAsHex(encoded_message)

	// Set up the transaction parameters
	const beneficiary = {
		V3: {
			parents: 0,
			interior: {
				X4: [
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
					{
						GeneralIndex: accountNonce,
					},
				],
			},
		},
	}

	let assets
	let destination

	if (sourceIsAssetHub) {
		destination = {
			V3: {
				parents: 1,
				interior: {
					X1: {
						Parachain: params.paraId,
					},
				},
			},
		}

		assets = {
			V3: [
				{
					id: {
						Concrete: {
							parents: 1,
							interior: "Here",
						},
					},
					fun: {
						Fungible: parseUnits(params.amount.toString(), DECIMALS),
					},
				},
			],
		}
	} else {
		destination = {
			V3: {
				parents: 0,
				interior: {
					X1: {
						Parachain: params.paraId,
					},
				},
			},
		}

		assets = {
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
	}

	const feeAssetItem = 0
	const weightLimit = "Unlimited"

	let tx

	if (sourceIsAssetHub) {
		tx = sourceApi.tx.polkadotXcm.limitedReserveTransferAssets(
			destination,
			beneficiary,
			assets,
			feeAssetItem,
			weightLimit,
		)
	} else {
		tx = sourceApi.tx.xcmPallet.limitedReserveTransferAssets(
			destination,
			beneficiary,
			assets,
			feeAssetItem,
			weightLimit,
		)
	}

	let closed = false
	// Create the stream to report events
	let unsubscribe: () => void
	const stream = new ReadableStream<HyperbridgeTxEvents>(
		{
			async start(controller) {
				unsubscribe = await tx.signAndSend(who, options, async (result: any) => {
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
								message_id,
							})
						} else if (status.isInBlock || status.isFinalized) {
							// Send event with the status kind (either Dispatched or Finalized)
							controller.enqueue({
								kind: "Finalized",
								transaction_hash: txHash.toHex(),
								message_id,
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
