import { createStorage } from "unstorage"
// @ts-expect-error failed to resolve types
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
import type { CancellationStorageOptions, StorageDriverKey } from "@/storage/types"

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
