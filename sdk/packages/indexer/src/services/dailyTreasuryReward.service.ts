import { DailyTreasuryRelayerReward } from "@/configs/src/types"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { ENV_CONFIG } from "@/constants"
import { Struct, u32, u128, bool, _void, Enum, u8, Vector } from "scale-ts"
import { hexToBytes } from "viem"
import { xxhashAsHex, blake2AsU8a, decodeAddress, xxhashAsU8a } from "@polkadot/util-crypto"
import fetch from "node-fetch"
import { timestampToDate } from "@/utils/date.helpers"
import { AccountInfo } from "@/services/bridgeTokenSupply.service"
import { getStateId, StateMachine } from "@/utils/state-machine.helper"
import { retryPromise } from "@/utils/retry.utils"
import { compactAddLength } from "@polkadot/util"

const REPUTATION_ASSET_ID = "0x0000000000000000000000000000000000000000000000000000000000000001"
export const TREASURY_ADDRESS = "13UVJyLkyUpEiXBx5p776dHQoBuuk3Y5PYp5Aa89rYWePWA3"
interface SubstrateStorageResponse {
	jsonrpc: "2.0"
	id: number
	result?: string
	error?: { message: string }
}

const AssetAccount = Struct({
	balance: u128,
	isFrozen: bool,
	reason: _void,
	extra: _void,
})

export class DailyTreasuryRewardService {
	/**
	 * Finds the daily treasury reward record for a given date and adds to the amount
	 * Creates a new record if one doesn't exist.
	 */
	static async update(date: bigint, amount: bigint): Promise<void> {
		const day = timestampToDate(date)
		day.setUTCHours(0, 0, 0, 0)
		const id = day.toISOString().slice(0, 10)

		let record = await DailyTreasuryRelayerReward.get(id)

		if (!record) {
			record = DailyTreasuryRelayerReward.create({
				id: id,
				date: day,
				dailyRewardAmount: BigInt(0),
			})
		}

		record.dailyRewardAmount += amount
		await record.save()
	}

	private static async getStorage(storageKey: string, logMessage: string): Promise<string | null> {
		const operation = async (): Promise<string | null> => {
			const hyperbridgeChain = getHostStateMachine(chainId);
			const rpcUrl = replaceWebsocketWithHttp(ENV_CONFIG[hyperbridgeChain] || "");
			if (!rpcUrl) {
				throw new Error(`No RPC URL found for Hyperbridge chain: ${hyperbridgeChain}`);
			}

			const response = await fetch(rpcUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					jsonrpc: "2.0",
					id: 1,
					method: "state_getStorage",
					params: [storageKey],
				}),
			});

			if (!response.ok) {
				throw new Error(`RPC request failed with status ${response.status}`);
			}

			const result: SubstrateStorageResponse = await response.json();

			if (result.error) {
				throw new Error(result.error.message);
			}

			return result.result || null;
		};

		try {
			return await retryPromise(operation, {
				maxRetries: 3,
				backoffMs: 1000,
				logMessage: `Fetch ${logMessage}`,
			});
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : String(e);
			logger.error(`All retries failed for ${logMessage}: ${errorMessage}`);
			return null;
		}
	}

	/**
	 * Fetches reputation asset balance for a given relayer account
	 */
	static async getReputationAssetBalance(accountId: string): Promise<bigint> {
		try {
			const storageKey = this.generateAssetsAccountStorageKey(REPUTATION_ASSET_ID, accountId)
			logger.info(`storage key is ${storageKey}`)
			const result = await this.getStorage(storageKey, "reputation asset balance")

			if (!result) {
				return BigInt(0)
			}

			const bytes = hexToBytes(result as `0x${string}`)
			const decoded = AssetAccount.dec(bytes)

			return decoded.balance
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : String(e)
			logger.error(`Failed to fetch reputation asset balance for ${accountId}: ${errorMessage}`)
			return BigInt(0)
		}
	}

	/**
	 * Fetches BRIDGE balance for the treasury
	 */
	static async getTreasuryBalance(): Promise<bigint> {
		try {
			const storageKey = this.generateSystemAccountStorageKey(TREASURY_ADDRESS)
			const result = await this.getStorage(storageKey, "treasury balance")

			if (!result) {
				return BigInt(0)
			}

			const bytes = hexToBytes(result as `0x${string}`)
			const decoded = AccountInfo.dec(bytes)

			return decoded.data.free
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : String(e)
			logger.error(`Failed to fetch treasury balance: ${errorMessage}`)
			return BigInt(0)
		}
	}

	/**
	 * Get Fee Token Decimal stored on chain
	 */
	static async getFeeTokenDecimals(stateMachine: any): Promise<number> {
		try {

			const storageKey = this.generateFeeTokenDecimalsStorageKey(stateMachine)
			logger.info(`storage key is ${storageKey}`)
			const result = await this.getStorage(storageKey, "fee token decimals")
			if (!result) {
				logger.warn(`No fee token decimals found for state machine: ${JSON.stringify(stateMachine)}`)
				return 18
			}
			logger.info(`fee token decimal result  is ${result}`)

			const bytes = hexToBytes(result as `0x${string}`)
			return u8.dec(bytes)
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : String(e)
			logger.error(`Failed to fetch fee token decimals: ${errorMessage}`)
			return 18
		}
	}

	/**
	 * Sorage Key for fee token decimal
	 */
	private static generateFeeTokenDecimalsStorageKey(stateMachine: any): string {
		const palletHash = xxhashAsHex("HostExecutive", 128)
		const storageHash = xxhashAsHex("FeeTokenDecimals", 128)

		logger.info(`trying to encode stateMachine:  ${JSON.stringify(stateMachine)}`)
		let stateId = getStateId(stateMachine)
		const encodedKey = StateMachine.enc(stateId)
		logger.info(`encoded stateMachine: ${encodedKey}`)

		const keyHash = blake2AsU8a(encodedKey, 128)

		logger.info(`encoded stateMachine keyHash: ${keyHash}`)

		const finalKey = new Uint8Array([
			...hexToBytes(palletHash),
			...hexToBytes(storageHash),
			...keyHash,
			...encodedKey,
		])

		logger.info(`encoded stateMachine finalKey: ${finalKey}`)

		return `0x${Buffer.from(finalKey).toString("hex")}`
	}

	/**
	 * Generates the assets account stprage key
	 */
	private static generateAssetsAccountStorageKey(assetId: `0x${string}`, accountId: string): string {
		const palletHash = xxhashAsHex("Assets", 128)
		const storageHash = xxhashAsHex("Account", 128)

		const assetIdBytes = hexToBytes(assetId)
		const accountIdBytes = decodeAddress(accountId)

		const assetIdHashed = blake2AsU8a(assetIdBytes, 128)
		const accountIdHashed = blake2AsU8a(accountIdBytes, 128)

		const finalKey = new Uint8Array([
			...hexToBytes(palletHash),
			...hexToBytes(storageHash),
			...assetIdHashed,
			...assetIdBytes,
			...accountIdHashed,
			...accountIdBytes,
		])

		return `0x${Buffer.from(finalKey).toString("hex")}`
	}

	/**
	 *
	 * Generates System Account storage Key
	 */
	private static generateSystemAccountStorageKey(accountId: string): string {
		const palletHash = xxhashAsHex("System", 128)
		const storageHash = xxhashAsHex("Account", 128)
		const accountIdBytes = decodeAddress(accountId)
		const accountIdHashed = blake2AsU8a(accountIdBytes, 128)

		const finalKey = new Uint8Array([
			...hexToBytes(palletHash),
			...hexToBytes(storageHash),
			...accountIdHashed,
			...accountIdBytes,
		])

		return `0x${Buffer.from(finalKey).toString("hex")}`
	}
}
