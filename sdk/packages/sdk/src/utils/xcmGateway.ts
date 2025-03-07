import type { ApiPromise } from "@polkadot/api"
import type { HexString } from "@/types"
import type { SignerOptions } from "@polkadot/api/types"
import { u8aToHex } from "@polkadot/util"
import { decodeAddress } from "@polkadot/util-crypto"
import { parseUnits } from "viem"
import type {EventRecord, Header} from "@polkadot/types/interfaces"
import type { ISubmittableResult } from "@polkadot/types/types"

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
			events: ISubmittableResult["events"]
	  }
	| {
			kind: "Finalized"
			transaction_hash: HexString
			events: ISubmittableResult["events"]
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
	amount: bigint

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
 * using XCM (Cross-Consensus Message Format).
 *
 * This function initiates a teleport transaction, monitors its status, and yields events
 * about the transaction's progress through an AsyncGenerator. It handles the complete
 * lifecycle of a teleport operation:
 * 1. Transaction preparation and signing
 * 2. Broadcasting to the relay chain
 * 3. Tracking the transaction until it's included in a block
 * 4. Monitoring  Hyperbridge for the commitment hash
 *
 * @param relayApi - Polkadot API instance connected to the relay chain
 * @param hyperbridge - Polkadot API instance connected to the Hyperbridge parachain
 * @param who - Sender's SS58Address address
 * @param options - Transaction signing options
 * @param params - Teleport parameters including destination, recipient, and amount
 * @yields {HyperbridgeTxEvents} Stream of events indicating transaction status
 * @throws {Error} If there's an issue getting the Hyperbridge block or other failures
 */
export async function teleportDot(
	relayApi: ApiPromise,
	hyperbridge: ApiPromise,
	who: string,
	options: Partial<SignerOptions>,
	params: XcmGatewayParams,
): Promise<ReadableStream<HyperbridgeTxEvents>> {
	// 2. initiate the transaction
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
							id: u8aToHex(decodeAddress(who, false, 42)),
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
	const finalizedHash = await hyperbridge.rpc.chain.getFinalizedHead()
	const finalizedHead = await hyperbridge.rpc.chain.getHeader(finalizedHash)
	const hyperbridgeBlock = finalizedHead.number.toNumber()

	if (!hyperbridgeBlock) throw new Error("Error getting Hyperbridge Block")

	let unsubscribe: () => void
	const stream = new ReadableStream<HyperbridgeTxEvents>(
		{
			async start(controller) {
				unsubscribe = await tx.signAndSend(who, options, async (result) => {
					try {
						const { status, dispatchError, txHash } = result
						// @ts-expect-error Type Mismatch
						const events = result.events as ISubmittableResult['events'];

						if (dispatchError) {
							controller.enqueue({
								kind: "Error",
								error: `Error watching extrinsic: ${dispatchError.toString()}`,
							})
							return
						}

						if (status.isReady) {
							// send tx hash as soon as it is available
							controller.enqueue({
								kind: "Ready",
								transaction_hash: txHash.toHex(),
							})
						} else if (status.isInBlock) {
							await relayApi.rpc.chain.getHeader(status.asInBlock)
							const { commitment, block_number } = await watchForRequestCommitment(
								hyperbridge,
								who,
								params,
								hyperbridgeBlock,
							)

							// send block number once available
							controller.enqueue({
								kind: "Dispatched",
								block_number: block_number,
								transaction_hash: txHash.toHex(),
								commitment,
								events: events,
							})
						}

						if (status.isFinalized) {
							controller.enqueue({
								kind: "Finalized",
								transaction_hash: txHash.toHex(),
								events: events
							})
						}
					} catch (err) {
						controller.enqueue({
							kind: "Error",
							error: String(err),
						})
					}
				})
			},
			pull() {
				// We don't really need a pull in this example
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

// Watch for the request to be dispatched from hyperbridge
async function watchForRequestCommitment(
	hyperbridge: ApiPromise,
	who: string,
	params: XcmGatewayParams,
	start_block: number,
): Promise<{
	commitment: HexString
	block_number: bigint
	block_hash: HexString
}> {
	return new Promise((resolve, reject) => {
		let blockCount = 0
		let last_block = start_block
		let unsubscribeHyperbridgeEvents = () => {}

		const handleLatestHeader = async (lastHeader: Header) => {
			const finalized = lastHeader.number.toNumber()
			blockCount += Math.max(finalized - last_block, 1)
			for (let block_number = last_block; block_number <= finalized; block_number++) {
				const block_hash = await hyperbridge.rpc.chain.getBlockHash(block_number)

				// just to be safe, query events at this specific hash
				const apiAt = await hyperbridge.at(block_hash)
				const events = (await apiAt.query.system.events()) as unknown as EventRecord[]

				for (const record of events) {
					const { event } = record

					if (event.section === "xcmGateway" && event.method === "AssetTeleported") {
						const commitment = extractCommitmentHashFromEvent({
							record,
							from: who,
							params,
						})

						if (commitment) {
							unsubscribeHyperbridgeEvents?.()
							resolve({
								commitment,
								block_hash: block_hash.toHex(),
								block_number: BigInt(block_number),
							})
						}
					}
				}
			}

			last_block = finalized + 1

			if (blockCount >= 30) {
				unsubscribeHyperbridgeEvents?.()
				reject(new Error("No commitment received"))
			}
		}

		hyperbridge.rpc.chain
			.subscribeFinalizedHeads((header) => {
				// @ts-expect-error Issue referencing type
				return handleLatestHeader(header)
			})
			.then((unsubscribe) => {
				unsubscribeHyperbridgeEvents = unsubscribe
			})
			.catch(reject)
	})
}


/**
 * Extracts the commitment hash from the event data if the event data
 * matches the expected data
 */
export function extractCommitmentHashFromEvent({
	record,
	from: who,
	params,
}: {
	record: EventRecord
	from: string
	params: Pick<XcmGatewayParams, "destination" | "recipient">
}): HexString | undefined {
	const { event } = record

	const [from, to, _amount, dest, commitment] = event.data

	const decodedFrom = u8aToHex(decodeAddress(from.toString(), false))
	const decodedWho = u8aToHex(decodeAddress(who, false))
	const isExpectedEvent =
		decodedFrom === decodedWho &&
		to.toString().toLowerCase() === params.recipient.toLowerCase() &&
		dest.toString().includes(params.destination?.toString())

	if (!isExpectedEvent) {
		throw new Error("Error extracting commitment. Data mismatch")
	}

	return commitment.toString() as HexString
}
