import { chainByChainId } from "@/cli/init/chains"
import { formatChainKey } from "@/config/interpolated-curve"
import type { FillerConfigService, ResolvedChainConfig } from "@/services/FillerConfigService"
import { moduleLogger, type Logger } from "@/services/Logger"

export const BUNDLER_PREFLIGHT_TIMEOUT_MS = 10_000

export interface BundlerPreflightOptions {
	timeoutMs?: number
	/** Chains marked true here never send a UserOperation, so their bundler is not asked. */
	watchOnly?: Record<number, boolean>
}

/**
 * Refuses any chain whose bundler answers eth_supportedEntryPoints without the
 * EntryPoint that chain's SolverAccount validates against, since that bundler
 * rejects every fill UserOperation. A bundler that cannot be asked, or whose
 * answer cannot be read, is only warned about: that says nothing about which
 * EntryPoints it serves. Watch-only chains, chains without a bundler, and chains
 * without a known EntryPoint are left to the fill path.
 */
export async function assertBundlersServeEntryPoint(
	chains: Pick<ResolvedChainConfig, "chainId" | "bundlerUrl">[],
	configService: Pick<FillerConfigService, "getEntryPointAddress" | "loggers">,
	options: BundlerPreflightOptions = {},
): Promise<void> {
	const timeoutMs = options.timeoutMs ?? BUNDLER_PREFLIGHT_TIMEOUT_MS
	const logger = moduleLogger(configService.loggers, "bundler-preflight")
	const failures = await Promise.all(
		chains.map(async ({ chainId, bundlerUrl }) => {
			if (options.watchOnly?.[chainId] === true) return undefined
			if (!bundlerUrl?.trim()) return undefined
			const entryPoint = configService.getEntryPointAddress(formatChainKey(chainId))
			if (!entryPoint) return undefined
			return checkBundler(chainId, bundlerUrl.trim(), entryPoint, timeoutMs, logger)
		}),
	)
	const messages = failures.filter((failure): failure is string => failure !== undefined)
	if (messages.length > 0) throw new Error(messages.join("; "))
}

export function servesEntryPoint(listed: unknown, entryPoint: string): boolean {
	return Array.isArray(listed) && listed.some((address) => String(address).toLowerCase() === entryPoint.toLowerCase())
}

export function describeEntryPoints(listed: unknown): string {
	return Array.isArray(listed) && listed.length > 0 ? listed.map(String).join(", ") : "none"
}

/** The host alone: bundler URLs carry API keys in their paths and query strings. */
export function bundlerHost(url: string): string {
	try {
		return new URL(url).host || "an unparseable URL"
	} catch {
		return "an unparseable URL"
	}
}

function chainName(chainId: number): string {
	const label = chainByChainId(chainId)?.label
	return label ? `${label} (${formatChainKey(chainId)})` : formatChainKey(chainId)
}

async function checkBundler(
	chainId: number,
	bundlerUrl: string,
	entryPoint: string,
	timeoutMs: number,
	logger: Logger,
): Promise<string | undefined> {
	const host = bundlerHost(bundlerUrl)
	const subject = `Bundler ${host} for ${chainName(chainId)}`
	const signal = AbortSignal.timeout(timeoutMs)
	let listed: string[]
	try {
		listed = await supportedEntryPoints(bundlerUrl, signal)
	} catch (err) {
		const reason = signal.aborted ? `timed out after ${timeoutMs}ms` : errorReason(err)
		logger.warn(
			{ chain: formatChainKey(chainId), bundler: host },
			`${subject} did not answer eth_supportedEntryPoints: ${reason.split(bundlerUrl).join(host)}. ` +
				`Continuing without confirming it serves EntryPoint ${entryPoint}`,
		)
		return undefined
	}
	if (servesEntryPoint(listed, entryPoint)) return undefined
	return `${subject} does not support EntryPoint ${entryPoint}; it lists ${describeEntryPoints(listed)}`
}

async function supportedEntryPoints(bundlerUrl: string, signal: AbortSignal): Promise<string[]> {
	const response = await fetch(bundlerUrl, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "eth_supportedEntryPoints", params: [] }),
		signal,
	})
	if (!response.ok) throw new Error(`HTTP ${response.status}`)
	const json = (await response.json()) as { result?: unknown; error?: { message?: string } }
	if (json.error) throw new Error(json.error.message ?? JSON.stringify(json.error))
	const { result } = json
	if (!Array.isArray(result) || !result.every((address) => typeof address === "string")) {
		throw new Error("the result is not a list of addresses")
	}
	return result
}

function errorReason(err: unknown): string {
	if (!(err instanceof Error)) return String(err)
	// Node's fetch reports every network failure as "fetch failed" and keeps the useful part in `cause`.
	const { cause } = err as { cause?: unknown }
	return cause instanceof Error ? `${err.message} (${cause.message})` : err.message
}
