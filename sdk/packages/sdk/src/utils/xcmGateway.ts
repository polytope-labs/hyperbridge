import type { HexString } from "@/types"
import { StateMachine } from "@/utils"
import type { ApiPromise } from "@polkadot/api"
import type { SignerOptions } from "@polkadot/api/types"
import { hexToU8a, u8aToHex } from "@polkadot/util"
import { decodeAddress, keccakAsHex } from "@polkadot/util-crypto"
import { Bytes, Struct, u64 } from "scale-ts"
import { parseUnits } from "viem"

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
 * Parameters for teleporting DOT from AssetHub to EVM-based destination
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
 * Teleports DOT tokens from AssetHub to Hyperbridge parachain
 * using XCM V3 with transferAssetsUsingTypeAndThen.
 *
 * This function uses transferAssetsUsingTypeAndThen to construct XCM V3 transfers with a custom
 * beneficiary structure that embeds Hyperbridge-specific parameters (sender account, recipient EVM address,
 * timeout, and nonce) within an X4 junction. The beneficiary is wrapped in a DepositAsset XCM V3 instruction
 * that deposits all transferred assets. The assets are transferred using LocalReserve transfer type.
 *
 * It handles the complete lifecycle of a teleport operation:
 * 1. Encoding Hyperbridge parameters into the beneficiary X4 junction
 * 2. Wrapping the beneficiary in a DepositAsset XCM V3 instruction using sourceApi.createType
 * 3. Constructing the XCM V3 transfer transaction using polkadotXcm.transferAssetsUsingTypeAndThen
 * 4. Transaction signing and broadcasting
 * 5. Yielding events about transaction status through a ReadableStream
 *
 * Note: There is no guarantee that both Dispatched and Finalized events will be yielded.
 * Consumers should listen for either one of these events instead of expecting both.
 *
 * @param sourceApi - Polkadot API instance connected to AssetHub
 * @param who - Sender's SS58Address address
 * @param options - Transaction signing options
 * @param params - Teleport parameters including destination, recipient, amount, timeout, and paraId
 * @yields {HyperbridgeTxEvents} Stream of events indicating transaction status
 */
export async function teleportDot(param_: {
	sourceApi: ApiPromise
	who: string
	xcmGatewayParams: XcmGatewayParams
	options: Partial<SignerOptions>
}): Promise<ReadableStream<HyperbridgeTxEvents>> {
	const { sourceApi, who, options, xcmGatewayParams: params } = param_
	const { nonce: accountNonce } = (await sourceApi.query.system.account(who)) as any

	const encoded_message = MultiAccount.enc({
		substrate_account: decodeAddress(who),
		evm_account: hexToU8a(params.recipient),
		dest_state_machine: { tag: "Evm", value: params.destination },
		timeout: params.timeout,
		account_nonce: accountNonce,
	})

	const message_id = keccakAsHex(encoded_message)

	// Set up the custom beneficiary with embedded Hyperbridge parameters
	const beneficiary = {
		V4: {
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

	// AssetHub -> Hyperbridge parachain destination and assets
	const destination = {
		V4: {
			parents: 1,
			interior: {
				X1: [{ Parachain: params.paraId }],
			},
		},
	}

	const assets = {
		V4: [
			{
				id: {
					parents: 1,
					interior: "Here",
				},
				fun: {
					Fungible: parseUnits(params.amount.toString(), DECIMALS),
				},
			},
		],
	}

	const weightLimit = "Unlimited"

	// Fee asset ID must be wrapped with V4 version header as VersionedAssetId
	const feeAssetId = {
		V4: assets.V4[0].id,
	}

	// Wrap beneficiary in DepositAsset XCM instruction as required by transferAssetsUsingTypeAndThen
	// This instruction deposits all transferred assets to the custom beneficiary
	const customXcmOnDest = {
		V4: [
			{
				DepositAsset: {
					assets: {
						Wild: {
							AllCounted: 1,
						},
					},
					beneficiary: beneficiary.V4,
				},
			},
		],
	}

	// Use transferAssetsUsingTypeAndThen for AssetHub -> Hyperbridge transfer
	// This method allows us to specify custom beneficiary with embedded Hyperbridge parameters
	// TransferType: LocalReserve means assets are held in reserve on the source chain (AssetHub)
	const tx = sourceApi.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
		destination,
		assets,
		{ LocalReserve: null }, // Assets transfer type
		feeAssetId, // Fee asset ID wrapped as VersionedAssetId
		{ LocalReserve: null }, // Remote fee transfer type
		customXcmOnDest, // XCM instruction with DepositAsset containing custom beneficiary
		weightLimit,
	)

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
