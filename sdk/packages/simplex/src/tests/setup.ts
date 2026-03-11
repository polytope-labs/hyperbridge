import * as dotenv from "dotenv"
import * as path from "node:path"

const root = process.cwd()
dotenv.config({ path: path.resolve(root, "../../.env.local") })
