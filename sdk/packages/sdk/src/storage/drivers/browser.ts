import indexedDBDriver from "unstorage/drivers/indexedb"
import localStorageDriver from "unstorage/drivers/localstorage"
import type { LoadDriver } from "../types"

const BASE_KEY = "hyperbridge/sdk/proof"

export const loadDriver: LoadDriver = ({ key }) => {
	if (key === "localstorage") {
		return localStorageDriver({ base: BASE_KEY })
	}

	if (key === "indexeddb") {
		return indexedDBDriver({ base: BASE_KEY })
	}

	console.warn(
		`Hyperbridge/SDK/BrowserDriver: Unexpected storage driver: ${key}. Driver can't be loaded in the browser environment.`,
	)

	return null
}
