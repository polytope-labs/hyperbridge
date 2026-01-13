import { createStorage } from "unstorage"
// @ts-ignore - unstorage types don't resolve due to package.json exports
import inMemoryDriver from "unstorage/drivers/memory"
import { loadDriver } from "@/storage/load-driver"
import { bytesToHex, hexToBytes } from "viem"
import { GetRequest, Proof } from "@/utils/substrate"
import {
	convertCodecToIGetRequest,
	convertIGetRequestToCodec,
	convertCodecToIProof,
	convertIProofToCodec,
} from "@/chains/substrate"
import type { IGetRequest, HexString } from "@/types"
import type { IProof } from "@/chain"
import type { CancellationStorageOptions, SessionKeyStorageOptions, StorageDriverKey } from "@/storage/types"

/**
 * Encode IGetRequest to hex string using scale codec
 */
function encodeIGetRequest(request: IGetRequest): string {
	const codec = convertIGetRequestToCodec(request)
	const encoded = GetRequest.enc(codec)
	return bytesToHex(encoded)
}

/**
 * Decode hex string back to IGetRequest using scale codec
 */
function decodeIGetRequest(hex: string): IGetRequest {
	const bytes = hexToBytes(hex as HexString)
	const decoded = GetRequest.dec(bytes)
	return convertCodecToIGetRequest(decoded)
}

/**
 * Encode IProof to hex string using scale codec
 */
function encodeIProof(proof: IProof): string {
	const codec = convertIProofToCodec(proof)
	const encoded = Proof.enc(codec)
	return bytesToHex(encoded)
}

/**
 * Decode hex string back to IProof using scale codec
 */
function decodeIProof(hex: string): IProof {
	const bytes = hexToBytes(hex as HexString)
	const decoded = Proof.dec(bytes)
	return convertCodecToIProof(decoded)
}

const detectEnvironment = (): StorageDriverKey => {
	if (typeof process !== "undefined" && !!process.versions?.node) return "node"
	if (typeof globalThis !== "undefined" && "localStorage" in globalThis) return "localstorage"
	if (typeof globalThis !== "undefined" && "indexedDB" in globalThis) return "indexeddb"
	return "memory"
}

export function createCancellationStorage(options: CancellationStorageOptions = {}) {
	const key = options.env ?? detectEnvironment()
	const driver = loadDriver({ key, options }) ?? inMemoryDriver()
	const baseStorage = createStorage({ driver })

	const getItem = async <T>(key: string): Promise<T | null> => {
		const value = await baseStorage.getItem<string>(key)
		if (!value) return null

		if (key.includes("getRequest")) {
			const decoded = decodeIGetRequest(value)
			return decoded as T
		}

		if (key.includes("Proof")) {
			const decoded = decodeIProof(value)
			return decoded as T
		}

		throw new Error(`Unknown storage key type: ${key}`)
	}

	const setItem = async (key: string, value: unknown): Promise<void> => {
		if (key.includes("getRequest") && value && typeof value === "object") {
			const encoded = encodeIGetRequest(value as IGetRequest)
			await baseStorage.setItem(key, encoded)
			return
		}

		if (key.includes("Proof") && value && typeof value === "object") {
			const encoded = encodeIProof(value as IProof)
			await baseStorage.setItem(key, encoded)
			return
		}

		throw new Error(`Unknown storage key type: ${key}`)
	}

	const removeItem = async (key: string): Promise<void> => {
		await baseStorage.removeItem(key)
	}

	return Object.freeze({
		...baseStorage,
		getItem,
		setItem,
		removeItem,
	})
}

export const STORAGE_KEYS = Object.freeze({
	destProof: (orderId: string) => `cancel-order:${orderId}:destProof`,
	getRequest: (orderId: string) => `cancel-order:${orderId}:getRequest`,
	sourceProof: (orderId: string) => `cancel-order:${orderId}:sourceProof`,
})

/**
 * Session key data stored for each order
 */
export interface SessionKeyData {
	/**
	 * The private key as a hex string
	 */
	privateKey: HexString

	/**
	 * The derived public address
	 */
	address: HexString

	/**
	 * The order commitment this session key is associated with
	 */
	commitment: HexString

	/**
	 * Timestamp when the session key was created
	 */
	createdAt: number
}

/**
 * Creates a session key storage instance for IntentGatewayV2 orders.
 * The storage is used to persist session key private keys so they can be
 * used later to sign solver selection messages.
 *
 * @param options - Optional configuration for the storage driver
 * @returns A storage instance with methods to get, set, and remove session keys
 */
export function createSessionKeyStorage(options: SessionKeyStorageOptions = {}) {
	const key = options.env ?? detectEnvironment()
	const driver = loadDriver({ key, options }) ?? inMemoryDriver()
	const baseStorage = createStorage({ driver })

	const SESSION_KEY_PREFIX = "session-key:"

	/**
	 * Gets a session key by order commitment
	 */
	const getSessionKey = async (commitment: HexString): Promise<SessionKeyData | null> => {
		const storageKey = `${SESSION_KEY_PREFIX}${commitment}`
		const value = await baseStorage.getItem<string>(storageKey)
		if (!value) return null

		try {
			return JSON.parse(value) as SessionKeyData
		} catch {
			return null
		}
	}

	/**
	 * Stores a session key for an order commitment
	 */
	const setSessionKey = async (commitment: HexString, data: SessionKeyData): Promise<void> => {
		const storageKey = `${SESSION_KEY_PREFIX}${commitment}`
		await baseStorage.setItem(storageKey, JSON.stringify(data))
	}

	/**
	 * Removes a session key by order commitment
	 */
	const removeSessionKey = async (commitment: HexString): Promise<void> => {
		const storageKey = `${SESSION_KEY_PREFIX}${commitment}`
		await baseStorage.removeItem(storageKey)
	}

	/**
	 * Lists all stored session keys
	 */
	const listSessionKeys = async (): Promise<SessionKeyData[]> => {
		const keys = await baseStorage.getKeys(SESSION_KEY_PREFIX)
		const sessionKeys: SessionKeyData[] = []

		for (const key of keys) {
			const value = await baseStorage.getItem<string>(key)
			if (value) {
				try {
					sessionKeys.push(JSON.parse(value) as SessionKeyData)
				} catch {
					// Skip invalid entries
				}
			}
		}

		return sessionKeys
	}

	return Object.freeze({
		getSessionKey,
		setSessionKey,
		removeSessionKey,
		listSessionKeys,
	})
}

/**
 * Storage keys for session key storage
 */
export const SESSION_KEY_STORAGE_KEYS = Object.freeze({
	sessionKey: (commitment: string) => `session-key:${commitment}`,
})
