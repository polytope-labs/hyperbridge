import type { Driver } from "unstorage"

export type StorageDriverKey = "node" | "localstorage" | "indexeddb" | "memory"

export interface CancellationStorageOptions {
	env?: StorageDriverKey
	basePath?: string
}

export interface SessionKeyStorageOptions {
	env?: StorageDriverKey
	basePath?: string
}

export type LoadDriver = ({ key }: { key: StorageDriverKey; options?: CancellationStorageOptions }) => Driver | null
