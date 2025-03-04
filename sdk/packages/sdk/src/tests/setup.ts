process.env.NODE_ENV = "test"

import * as dotenv from "dotenv"
import * as path from "path"

const root = process.cwd()
dotenv.config({ path: path.resolve(root, "../../.env") })
