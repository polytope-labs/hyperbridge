import { readFile } from "node:fs/promises"
import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const desktop = JSON.parse(await readFile(resolve(root, "package.json"), "utf8"))
const simplex = JSON.parse(await readFile(resolve(root, "../simplex/package.json"), "utf8"))
if (desktop.version !== simplex.version) {
	throw new Error(`Desktop version ${desktop.version} must match @hyperbridge/simplex ${simplex.version}`)
}
