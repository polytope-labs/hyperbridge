import { BridgeTokenSupply } from "@/configs/src/types"
import { normalizeTimestamp, timestampToDate } from "@/utils/date.helpers"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { ENV_CONFIG } from "@/constants"
import fetch from "node-fetch"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Codec, Vector, Struct, u128, u8, Tuple, u64, u32 } from "scale-ts"
import { hexToBytes } from "viem"

const FOUNDATION = "1e3df2cbfa10ceadc854bd5aa305462c2f81bbc81cc7f6ef462ce857bd0c90b3"
const TEAM = "5525104d00c439213dcbf428b867f0930754f100b75873382f2103ce5f846163"

interface SubstrateStorageResponse {
	jsonrpc: "2.0"
	id: number
	result?: string
	error?: {
		code: number
		message: string
	}
}

interface SubstrateKeysResponse {
	jsonrpc: "2.0"
	id: number
	result?: string[]
	error?: {
		code: number
		message: string
	}
}

const AccountInfo = Struct({
	nonce: u32,
	consumers: u32,
	providers: u32,
	sufficients: u32,
	data: Struct({
		free: u128,
		reserve: u128,
		frozen: u128,
		flags: u128,
	}),
})

// Define the BalanceLock codec using scale-ts
// BalanceLock struct: [u8; 8] (lock ID), u128 (amount), enum (reasons)
const BalanceLock = Struct({
	id: Tuple(u8, u8, u8, u8, u8, u8, u8, u8), // [u8; 8] - 8-byte lock ID
	amount: u128, // u128 lock amount
	reasons: u8, // Simplified enum as u8 (in practice, this could be more complex)
})

// Vector of BalanceLocks
const BalanceLocksVec = Vector(BalanceLock)

/**
 * Bridge Token Supply Service manages token supply data for Hyperbridge
 */
export class BridgeTokenSupplyService {
	/**
	 * Updates token supply data by fetching circulating supply from Hyperbridge chain via RPC
	 * @param blockTimestamp - Block timestamp for when this update occurs
	 */
	static async updateTokenSupply(blockTimestamp: bigint): Promise<BridgeTokenSupply | Error> {
		try {
			// Get RPC URL for Hyperbridge chain
			const hyperbridgeChain = getHostStateMachine(chainId) // Use current chain ID from the indexer context
			const rpcUrl = replaceWebsocketWithHttp(ENV_CONFIG[hyperbridgeChain] || "")
			if (!rpcUrl) {
				throw new Error(`No RPC URL found for Hyperbridge chain: ${hyperbridgeChain}`)
			}

			// Fetch total supply from substrate balances pallet
			const totalSupplyResult = await this.getTotalSupply(rpcUrl)
			if (totalSupplyResult instanceof Error) {
				return totalSupplyResult
			}

			// Fetch circulating supply by subtracting locked/staked amounts
			const circulatingSupplyResult = await this.getCirculatingSupply(rpcUrl)
			if (circulatingSupplyResult instanceof Error) {
				return circulatingSupplyResult
			}

			// Scale down values by 12 decimal places for storage (convert from smallest unit to token units)
			// This converts from the raw blockchain units to human-readable token amounts
			const scaleFactor = BigInt(10) ** BigInt(12) // 10^12 = 1,000,000,000,000
			const scaledTotalSupply = totalSupplyResult / scaleFactor
			const scaledCirculatingSupply = circulatingSupplyResult / scaleFactor

			// Create or update the BridgeTokenSupply entity (single entity for Hyperbridge)
			const entityId = "hyperbridge-token-supply"
			let bridgeTokenSupply = await BridgeTokenSupply.get(entityId)

			const normalizedTimestamp = normalizeTimestamp(blockTimestamp)

			if (!bridgeTokenSupply) {
				bridgeTokenSupply = BridgeTokenSupply.create({
					id: entityId,
					totalSupply: scaledTotalSupply,
					circulatingSupply: scaledCirculatingSupply,
					lastUpdatedAt: normalizedTimestamp,
					createdAt: timestampToDate(blockTimestamp),
				})
			} else {
				bridgeTokenSupply.totalSupply = scaledTotalSupply
				bridgeTokenSupply.circulatingSupply = scaledCirculatingSupply
				bridgeTokenSupply.lastUpdatedAt = normalizedTimestamp
			}

			await bridgeTokenSupply.save()

			logger.info(
				`[BridgeTokenSupplyService.updateTokenSupply] Updated Hyperbridge token supply: total=${scaledTotalSupply} (raw: ${totalSupplyResult}), circulating=${scaledCirculatingSupply} (raw: ${circulatingSupplyResult})`,
			)

			return bridgeTokenSupply
		} catch (error) {
			logger.error(
				`[BridgeTokenSupplyService.updateTokenSupply] Failed to update Hyperbridge token supply`,
				error,
			)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Gets the total supply of the native token from substrate balances pallet
	 * @param rpcUrl - The RPC URL for the substrate chain
	 */
	private static async getTotalSupply(rpcUrl: string): Promise<bigint | Error> {
		try {
			// Storage key for balances.totalIssuance
			const storageKey = "0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"

			const response = await fetch(rpcUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					jsonrpc: "2.0",
					id: 1,
					method: "state_getStorage",
					params: [storageKey, null],
				}),
			})

			const result: SubstrateStorageResponse = await response.json()

			if (result.error) {
				throw new Error(`RPC error: ${result.error.message}`)
			}

			if (!result.result) {
				throw new Error("No result returned from RPC call")
			}

			// Convert hex string to Uint8Array using viem's hexToBytes
			const bytes = hexToBytes(result.result as `0x${string}`)

			// Decode u128 using scale-ts
			const totalSupply = u128.dec(bytes)

			return totalSupply
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.getTotalSupply] Failed to get total supply`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Calculates circulating supply by subtracting locked amounts from total supply
	 * Formula: Circulating Supply = Total Supply - (Sum of all account locks + Inactive Issuance)
	 * @param rpcUrl - The RPC URL for the substrate chain
	 */
	private static async getCirculatingSupply(rpcUrl: string): Promise<bigint | Error> {
		try {
			// Get total supply
			const totalSupply = await this.getTotalSupply(rpcUrl)
			if (totalSupply instanceof Error) {
				return totalSupply
			}

			// Get inactive issuance
			const inactiveIssuance = await this.getInactiveIssuance(rpcUrl)
			if (inactiveIssuance instanceof Error) {
				return inactiveIssuance
			}

			// Get team and foundation account
			const teamTotal = await this.getTeamAndFoundationBalance(rpcUrl)
			if (teamTotal instanceof Error) {
				return teamTotal
			}

			// Get sum of all account locks
			const totalLocks = await this.getTotalAccountLocks(rpcUrl)
			if (totalLocks instanceof Error) {
				return totalLocks
			}

			// Calculate circulating supply: Total Supply - (Inactive Issuance + Total Locks)
			const circulatingSupply = totalSupply - (inactiveIssuance + totalLocks + teamTotal)

			logger.debug(
				`[BridgeTokenSupplyService.getCirculatingSupply] Total: ${totalSupply}, Inactive: ${inactiveIssuance}, Locks: ${totalLocks}, Team and Foundation: ${teamTotal}, Circulating: ${circulatingSupply}`,
			)

			return circulatingSupply
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.getCirculatingSupply] Failed to get circulating supply`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Gets the inactive issuance from the balances pallet
	 * @param rpcUrl - The RPC URL for the substrate chain
	 */
	private static async getInactiveIssuance(rpcUrl: string): Promise<bigint | Error> {
		try {
			// Storage key for balances.inactiveIssuance
			const storageKey = "0xc2261276cc9d1f8598ea4b6a74b15c2f1ccde6872881f893a21de93dfe970cd5"

			const response = await fetch(rpcUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					jsonrpc: "2.0",
					id: 1,
					method: "state_getStorage",
					params: [storageKey, null],
				}),
			})

			const result: SubstrateStorageResponse = await response.json()

			if (result.error) {
				throw new Error(`RPC error: ${result.error.message}`)
			}

			// If no inactive issuance exists, return 0
			if (!result.result) {
				return BigInt(0)
			}

			// Convert hex string to Uint8Array using viem's hexToBytes
			const bytes = hexToBytes(result.result as `0x${string}`)

			// Decode u128 using scale-ts
			const inactiveIssuance = u128.dec(bytes)

			return inactiveIssuance
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.getInactiveIssuance] Failed to get inactive issuance`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Gets the inactive issuance from the balances pallet
	 * @param rpcUrl - The RPC URL for the substrate chain
	 */
	private static async getTeamAndFoundationBalance(rpcUrl: string): Promise<bigint | Error> {
		try {
			// Storage key for balances.inactiveIssuance
			const foundation_account_key =
				"0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9cc2c6dacd6a81b5cfa93c676700c2ccf1e3df2cbfa10ceadc854bd5aa305462c2f81bbc81cc7f6ef462ce857bd0c90b3"
			const team_account_key =
				"0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9a6f084e8a3b94abf6e45b5ee4bcc89db5525104d00c439213dcbf428b867f0930754f100b75873382f2103ce5f846163"

			// Query multiple storage keys at once
			const response = await fetch(rpcUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					jsonrpc: "2.0",
					id: 1,
					method: "state_queryStorageAt",
					params: [[foundation_account_key, team_account_key], null],
				}),
			})

			const result = await response.json()

			if (result.error) {
				throw new Error(`RPC error querying team and foundation accounts: ${result.error.message}`)
			}

			if (!result.result || !result.result.length) {
				return BigInt(0)
			}

			let batchTotal = BigInt(0)

			// Process each storage result
			for (const storageResult of result.result) {
				if (!storageResult.changes) continue

				for (const [key, value] of storageResult.changes) {
					if (!value) continue

					try {
						const amount = this.parseAccountBalance(value)
						batchTotal += amount
					} catch (parseError) {
						logger.warn(
							`[BridgeTokenSupplyService.processBatchLocks] Failed to parse lock data for key ${key}:`,
							parseError,
						)
						// Continue processing other locks even if one fails
					}
				}
			}

			return batchTotal
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.getInactiveIssuance] Failed to get inactive issuance`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Gets the sum of all account locks from the balances pallet locks storage map
	 * @param rpcUrl - The RPC URL for the substrate chain
	 */
	private static async getTotalAccountLocks(rpcUrl: string): Promise<bigint | Error> {
		try {
			// Storage key prefix for balances.locks map
			const storageKeyPrefix = "0xc2261276cc9d1f8598ea4b6a74b15c2f218f26c73add634897550b4003b26bc6"

			let totalLocks = BigInt(0)
			let startKey: string | null = null
			const pageSize = 1000
			const batchSize = 100

			// Keep fetching keys until we get all of them (pagination)
			while (true) {
				const keysResponse = await fetch(rpcUrl, {
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify({
						jsonrpc: "2.0",
						id: 1,
						method: "state_getKeysPaged",
						params: [storageKeyPrefix, pageSize, startKey, null],
					}),
				})

				const keysResult: SubstrateKeysResponse = await keysResponse.json()

				if (keysResult.error) {
					throw new Error(`RPC error getting keys: ${keysResult.error.message}`)
				}

				if (!keysResult.result || keysResult.result.length === 0) {
					// No more keys to fetch
					break
				}

				logger.debug(
					`[BridgeTokenSupplyService.getTotalAccountLocks] Fetched ${keysResult.result.length} keys in this page`,
				)

				// Process keys in batches to avoid overwhelming the RPC
				for (let i = 0; i < keysResult.result.length; i += batchSize) {
					const batch = keysResult.result.slice(i, i + batchSize)
					const batchLocks = await this.processBatchLocks(rpcUrl, batch)

					if (batchLocks instanceof Error) {
						logger.warn(
							`[BridgeTokenSupplyService.getTotalAccountLocks] Failed to process batch ${i}-${i + batchSize}:`,
							batchLocks,
						)
						continue
					}

					totalLocks += batchLocks
				}

				// If we got less than pageSize results, we've reached the end
				if (keysResult.result.length < pageSize) {
					break
				}

				// Set the start key for the next page (last key from current page)
				startKey = keysResult.result[keysResult.result.length - 1]
			}

			logger.debug(`[BridgeTokenSupplyService.getTotalAccountLocks] Total locks processed: ${totalLocks}`)

			return totalLocks
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.getTotalAccountLocks] Failed to get total account locks`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Processes a batch of lock storage keys and returns the sum of locked amounts
	 * @param rpcUrl - The RPC URL for the substrate chain
	 * @param keys - Array of storage keys to query
	 */
	private static async processBatchLocks(rpcUrl: string, keys: string[]): Promise<bigint | Error> {
		try {
			// Query multiple storage keys at once
			const response = await fetch(rpcUrl, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					jsonrpc: "2.0",
					id: 1,
					method: "state_queryStorageAt",
					params: [keys, null],
				}),
			})

			const result = await response.json()

			if (result.error) {
				throw new Error(`RPC error querying batch: ${result.error.message}`)
			}

			if (!result.result || !result.result.length) {
				return BigInt(0)
			}

			let batchTotal = BigInt(0)

			// Process each storage result
			for (const storageResult of result.result) {
				if (!storageResult.changes) continue

				for (const [key, value] of storageResult.changes) {
					if (!value) continue
					// Don't count team and foundation locks
					if (key.endsWith(FOUNDATION) || key.endsWith(TEAM)) {
						logger.info(`Skipping locks on team and foundation account`)
						continue
					}

					try {
						// Parse the encoded locks data
						// Substrate stores locks as Vec<BalanceLock<Balance>>
						// Each lock has an amount field we need to sum
						const lockAmount = this.parseLockData(value)
						if (lockAmount > 0) {
							batchTotal += lockAmount
						}
					} catch (parseError) {
						logger.warn(
							`[BridgeTokenSupplyService.processBatchLocks] Failed to parse lock data for key ${key}:`,
							parseError,
						)
						// Continue processing other locks even if one fails
					}
				}
			}

			return batchTotal
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.processBatchLocks] Failed to process batch locks`, error)
			return error instanceof Error ? error : new Error(String(error))
		}
	}

	/**
	 * Parses lock data from substrate storage to extract the locked amount using scale-ts
	 * BalanceLock struct format:
	 * - First 8 bytes: static byte array (lock ID)
	 * - Next 16 bytes: u128 lock amount (little-endian)
	 * - Next 1+ bytes: enum (reasons)
	 * @param hexData - Hex encoded lock data from substrate
	 */
	private static parseLockData(hexData: string): bigint {
		try {
			// Remove 0x prefix if present
			const cleanHex = hexData.startsWith("0x") ? hexData.slice(2) : hexData

			if (cleanHex.length === 0) {
				return BigInt(0)
			}

			// Convert hex string to Uint8Array using viem's hexToBytes
			const hexWithPrefix = cleanHex.startsWith("0x") ? cleanHex : `0x${cleanHex}`
			const bytes = hexToBytes(hexWithPrefix as `0x${string}`)

			if (bytes.length === 0) {
				return BigInt(0)
			}

			// Decode the Vec<BalanceLock> using scale-ts
			const decodedLocks = BalanceLocksVec.dec(bytes)

			// Sum all lock amounts
			let totalLocked = BigInt(0)
			for (const lock of decodedLocks) {
				totalLocked += lock.amount
			}

			logger.debug(
				`[BridgeTokenSupplyService.parseLockData] Decoded ${decodedLocks.length} locks, total amount: ${totalLocked}`,
			)

			return totalLocked
		} catch (error) {
			logger.error(`[BridgeTokenSupplyService.parseLockData] Failed to parse lock data: ${hexData}`, error)
			return BigInt(0)
		}
	}

	private static parseAccountBalance(hexData: string): bigint {
		try {
			// Remove 0x prefix if present
			const cleanHex = hexData.startsWith("0x") ? hexData.slice(2) : hexData

			if (cleanHex.length === 0) {
				return BigInt(0)
			}

			// Convert hex string to Uint8Array using viem's hexToBytes
			const hexWithPrefix = cleanHex.startsWith("0x") ? cleanHex : `0x${cleanHex}`
			const bytes = hexToBytes(hexWithPrefix as `0x${string}`)

			if (bytes.length === 0) {
				return BigInt(0)
			}

			const account = AccountInfo.dec(bytes)

			return account.data.free
		} catch (error) {
			logger.error(
				`[BridgeTokenSupplyService.parseAccountBalance] Failed to parse account info: ${hexData}`,
				error,
			)
			return BigInt(0)
		}
	}

	/**
	 * Gets the current Hyperbridge token supply data
	 */
	static async getTokenSupply(): Promise<BridgeTokenSupply | null> {
		const entityId = "hyperbridge-token-supply"
		return (await BridgeTokenSupply.get(entityId)) || null
	}
}
