import { describe, it, expect } from "vitest"
import * as dotenv from "dotenv"
import * as path from "node:path"

dotenv.config({ path: path.resolve(process.cwd(), "../../.env.local") })

const RPC_VARS = [
    "ETH_MAINNET", "BSC_MAINNET", "BASE_MAINNET",
    "POLYGON_MAINNET", "ARBITRUM_MAINNET",
]

describe("RPC Health Check", () => {
    it("should have RPC endpoints configured", () => {
        const configured = RPC_VARS.filter(k => !!process.env[k])
        console.log(`[health] RPC endpoints: ${configured.length}/${RPC_VARS.length}`)
        expect(configured.length).toBeGreaterThan(0)
    })
})
