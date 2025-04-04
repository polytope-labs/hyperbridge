import { hexToBytes, pad, bytesToHex } from "viem"
import type { ApiPromise } from "@polkadot/api"
import { Struct, Vector, u8, u64, _void, Option, u128, bool } from "scale-ts"
import { H256, StateMachine } from "@/utils/substrate"
import type { HexString } from "@/types"
import { convertStateMachineIdToEnum } from "@/chain"
import { keccakAsU8a, xxhashAsU8a } from "@polkadot/util-crypto"
import type { Option as PolakdotOption } from "@polkadot/types"
import type { EventRecord, StorageData } from "@polkadot/types/interfaces"
import type { SignerOptions } from "@polkadot/api/types"
import type { HyperbridgeTxEvents } from "./xcmGateway"

export type Params = {
	/** Asset symbol for the teleport operation */
	symbol: string
	/**
	 * Destination state machine identifier (e.g., "EVM-1", "SUBSTRATE-cere")
	 * that specifies the target blockchain or network
	 */
	destination: string
	/**
	 * Recipient address in hexadecimal format where the assets will be sent
	 * on the destination chain
	 */
	recipient: HexString
	/**
	 * Amount of tokens to teleport, represented as a bigint to handle
	 * large numeric values precisely
	 */
	amount: bigint
	/**
	 * Request timeout in block numbers or timestamp, after which the
	 * teleport operation will be considered failed
	 */
	timeout: bigint
	/**
	 * Address of the token gateway contract on the destination chain
	 * that will process the teleported assets
	 */
	tokenGatewayAddress: Uint8Array
	/**
	 * Fee paid to relayers who process the cross-chain transaction,
	 * represented as a bigint
	 */
	relayerFee: bigint
	/**
	 * Optional call data to be executed on the destination chain
	 * as part of the teleport operation
	 */
	callData?: Uint8Array
	/**
	 * Flag indicating whether to automatically redeem the tokens
	 * for erc20
	 */
	redeem: boolean
}

const TeleportParams = Struct({
	/// StateMachine
	destination: StateMachine,
	/// Receipient
	recepient: H256,
	/// Amount
	amount: u128,
	/// Request timeout
	timeout: u64,
	/// Token gateway address
	tokenGatewayAddress: Vector(u8),
	/// Relayer fee
	relayerFee: u128,
	/// Call data
	callData: Option(Vector(u8)),
	/// Redeem
	redeem: bool,
})

async function fetchLocalAssetId(params: { api: ApiPromise; assetId: Uint8Array }) {
	const { api, assetId } = params

	// twox_128
	const palletPrefix = xxhashAsU8a("TokenGateway", 128)
	// twox_128
	const storagePrefix = xxhashAsU8a("LocalAssets", 128)

	const full_key = new Uint8Array([...palletPrefix, ...storagePrefix, ...assetId])

	const hexKey = bytesToHex(full_key)

	// read account balance

	const storage_value: PolakdotOption<StorageData> = (await api.rpc.state.getStorage(
		hexKey,
	)) as PolakdotOption<StorageData>

	if (storage_value.isSome) {
		const assetId = storage_value.value.toU8a()

		return assetId
	}

	return null
}

/**
 * Teleports assets from Substrate to other chains via the token gateway
 *
 * Note: There is no guarantee that both Dispatched and Finalized events will be yielded.
 * Consumers should listen for either one of these events instead of expecting both.
 *
 *
 * @param apiPromise - Polkadot API instance
 * @param who - SS58Address
 * @param params - Teleport parameters
 * @param params.symbol - Asset symbol
 * @param params.destination - Target state machine ID
 * @param params.recipient - Recipient address
 * @param params.amount - Amount to teleport
 * @param params.timeout - Operation timeout
 * @param params.tokenGatewayAddress - Gateway contract address
 * @param params.relayerFee - Fee for the relayer
 * @param params.redeem - Whether to redeem on arrival
 * @param params.callData - Optional additional call data
 * @param options - Signer options
 * @yields {HyperbridgeTxEvents} Stream of events indicating transaction status
 * @throws Error when asset ID is unknown or transaction fails
 */
export async function teleport(teleport_param: {
	who: string
	params: Params
	apiPromise: ApiPromise
	options: Partial<SignerOptions>
}): Promise<ReadableStream<HyperbridgeTxEvents>> {
	const { who, options, params, apiPromise } = teleport_param

	const substrateComplianceAddr = (address: HexString, stateMachine: string) => {
		if (stateMachine.startsWith("EVM-")) return pad(address, { size: 32, dir: "left" })

		return address
	}

	const assetId = keccakAsU8a(params.symbol)

	// Fetch scale encoded local asset id

	const scaleEncodedAssetId = await fetchLocalAssetId({ api: apiPromise, assetId })

	if (scaleEncodedAssetId === null) {
		throw new Error("Unknown asset id provided")
	}

	const destination = convertStateMachineIdToEnum(params.destination)

	const recipient = hexToBytes(substrateComplianceAddr(params.recipient, params.destination))

	const teleportParams = {
		destination: destination,
		recepient: Array.from(recipient),
		amount: params.amount,
		timeout: BigInt(params.timeout),
		tokenGatewayAddress: Array.from(params.tokenGatewayAddress),
		relayerFee: BigInt(params.relayerFee),
		redeem: params.redeem,
		callData: params.callData ? Array.from(params.callData) : undefined,
	}

	const encoded = TeleportParams.enc(teleportParams)
	const fullCallData = new Uint8Array([...scaleEncodedAssetId, ...encoded])

	const tx = apiPromise.tx.tokenGateway.teleport(fullCallData)
	let unsub = () => {}
	let closed = false

	const stream = new ReadableStream<HyperbridgeTxEvents>(
		{
			async start(controller) {
				unsub = await tx.signAndSend(who, options, async (result) => {
					try {
						const { isInBlock, isError, dispatchError, txHash, isFinalized, status } = result
						// @ts-expect-error Type Mismatch
						const events = result.events as ISubmittableResult["events"]

						if (isError) {
							console.error("Transaction failed: ", dispatchError)
							controller.enqueue({ kind: "Error", error: dispatchError })
							unsub?.()
							controller.close()
							closed = true
							return
						}

						if (status.type === "Ready") {
							controller.enqueue({
								kind: "Ready",
								transaction_hash: txHash.toHex(),
							})
						}

						if (isInBlock || isFinalized) {
							const commitment_hash = readIsmpCommitmentHash(events)
							const blockHash = isInBlock ? status.asInBlock.toHex() : status.asFinalized.toHex()

							if (!commitment_hash) {
								controller.enqueue({
									kind: "Error",
									error: new Error("Commitment Hash missing"),
								})
								return controller.close()
							}

							const header = await apiPromise.rpc.chain.getHeader(blockHash)

							controller.enqueue({
								kind: isInBlock ? "Dispatched" : "Finalized",
								transaction_hash: txHash.toHex(),
								block_number: header.number.toBigInt(),
								commitment: commitment_hash,
							})

							if (isFinalized) {
								unsub?.()
								controller.close()
								closed = true
								return
							}
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
			cancel: () => unsub?.(),
		},
		{
			highWaterMark: 3,
			size: () => 1,
		},
	)

	return stream
}

function readIsmpCommitmentHash(events: EventRecord[]): HexString | undefined {
	for (const record of events) {
		const { event } = record

		if (event.section === "ismp" && event.method === "Request") {
			const commitment = event.data[3].toHex()

			if (!commitment) return

			return commitment
		}
	}
}
