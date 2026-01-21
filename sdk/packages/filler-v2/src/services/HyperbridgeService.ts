import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import type { SubmittableExtrinsic } from "@polkadot/api/types"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"

/**
 * Service for interacting with Hyperbridge via Polkadot.js
 * Handles bid submission to the pallet-intents coprocessor.
 * Maintains a persistent WebSocket connection for efficiency.
 *
 * Use the static `create()` method to instantiate.
 */
export class HyperbridgeService {
	private static logger = getLogger("hyperbridge-service")

	/**
	 * Creates a new HyperbridgeService with an established connection.
	 * WsProvider handles auto-reconnect internally.
	 *
	 * @param wsUrl - WebSocket URL for Hyperbridge
	 * @param substratePrivateKey - Private key for signing extrinsics
	 */
	static async create(wsUrl: string, substratePrivateKey: string): Promise<HyperbridgeService> {
		this.logger.debug({ wsUrl }, "Connecting to Hyperbridge")
		const provider = new WsProvider(wsUrl)
		const api = await ApiPromise.create({ provider })
		await api.isReady
		this.logger.info("Connected to Hyperbridge")

		return new HyperbridgeService(api, substratePrivateKey)
	}

	private constructor(
		private api: ApiPromise,
		private substratePrivateKey: string,
	) {}

	/**
	 * Disconnects from Hyperbridge.
	 * Should be called when the filler is stopping.
	 */
	async disconnect(): Promise<void> {
		HyperbridgeService.logger.debug("Disconnecting from Hyperbridge")
		await this.api.disconnect()
		HyperbridgeService.logger.debug("Disconnected from Hyperbridge")
	}

	/**
	 * Creates a Substrate keypair from the configured private key
	 * Supports both hex seed (without 0x prefix) and mnemonic phrases
	 */
	private getKeyPair() {
		const keyring = new Keyring({ type: "sr25519" })

		if (this.substratePrivateKey.includes(" ")) {
			return keyring.addFromMnemonic(this.substratePrivateKey)
		}
		const seedBytes = Buffer.from(this.substratePrivateKey, "hex")
		return keyring.addFromSeed(seedBytes)
	}

	/**
	 * Signs and sends an extrinsic, handling status updates and errors
	 */
	private async signAndSendExtrinsic(
		extrinsic: SubmittableExtrinsic<"promise">,
		successMessage: string,
		errorMessage: string,
	): Promise<BidSubmissionResult> {
		const keyPair = this.getKeyPair()

		return new Promise<BidSubmissionResult>((resolve) => {
			extrinsic
				.signAndSend(keyPair,  (status) => {
					if (status.isInBlock || status.isFinalized) {
						HyperbridgeService.logger.info(
							{
								blockHash: status.status.asInBlock.toHex(),
								extrinsicHash: extrinsic.hash.toHex(),
							},
							successMessage,
						)
						resolve({
							success: true,
							blockHash: status.status.asInBlock.toHex() as HexString,
							extrinsicHash: extrinsic.hash.toHex() as HexString,
						})
					} else if (status.isError) {
						HyperbridgeService.logger.error({ status: status.toHuman() }, errorMessage)
						resolve({
							success: false,
							error: `Extrinsic failed: ${status.status.toString()}`,
						})
					}
				})
				.catch((err: Error) => {
					HyperbridgeService.logger.error({ err }, errorMessage)
					resolve({
						success: false,
						error: err.message,
					})
				})
		})
	}

	/**
	 * Submits a bid to Hyperbridge's pallet-intents
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @param userOp - The encoded PackedUserOperation as hex string
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async submitBid(commitment: HexString, userOp: HexString): Promise<BidSubmissionResult> {
		try {
			HyperbridgeService.logger.info(
				{ commitment, userOpLength: userOp.length, signer: this.getKeyPair().address },
				"Submitting bid to Hyperbridge",
			)

			const extrinsic = this.api.tx.intents.placeBid(commitment, userOp)
			return await this.signAndSendExtrinsic(extrinsic, "Bid included in block", "Bid submission failed")
		} catch (error) {
			HyperbridgeService.logger.error({ err: error }, "Error submitting bid to Hyperbridge")
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Retracts a bid from Hyperbridge and reclaims the deposit
	 *
	 * Use this to remove unused quotes and claim back deposited BRIDGE tokens.
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async retractBid(commitment: HexString): Promise<BidSubmissionResult> {
		try {
			HyperbridgeService.logger.info(
				{ commitment, signer: this.getKeyPair().address },
				"Retracting bid from Hyperbridge",
			)

			const extrinsic = this.api.tx.intents.retractBid(commitment)
			return await this.signAndSendExtrinsic(
				extrinsic,
				"Bid retracted, deposit refunded",
				"Bid retraction failed",
			)
		} catch (error) {
			HyperbridgeService.logger.error({ err: error }, "Error retracting bid from Hyperbridge")
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}
}
