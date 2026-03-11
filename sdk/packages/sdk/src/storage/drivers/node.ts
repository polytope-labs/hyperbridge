// @ts-expect-error Unable to resolve type
import fsDriver from "unstorage/drivers/fs"
import type { LoadDriver } from "../types"

export const loadDriver: LoadDriver = ({ options }) => {
	return fsDriver({ base: options?.basePath ?? "./.hyperbridge-cache" })
}
