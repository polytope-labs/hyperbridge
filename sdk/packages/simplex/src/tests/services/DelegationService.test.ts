import { describe, it, expect } from "vitest"
import { erc20Abi } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { DelegationService } from "@/services/DelegationService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { FillerConfigService, type ResolvedChainConfig } from "@/services/FillerConfigService"
import { createPrivateKeySigningAccount } from "@/services/wallet/accounts/privatekey"

/**
 * Live integration test for `DelegationService.setupDelegationViaBundler` on Base mainnet.
 *
 * What it covers:
 *   - End-to-end paymaster-funded EIP-7702 delegation via a real ERC-4337 bundler.
 *   - The bundler reconstructs the EIP-7702 authorization hash and recovers the signer; if
 *     our local hash computation drifts from canonical RLP (e.g. encoding integer 0 as
 *     `0x00` instead of empty bytes), the recovered address won't match the UserOp sender
 *     and the bundler rejects the op. The Alchemy and Pimlico variants exercise the two
 *     gas-pricing branches in the service.
 *
 * Required env (suite skips when missing):
 *   BASE_MAINNET                 — Base mainnet RPC URL
 *   PRIVATE_KEY                  — EOA private key. Must hold ≥1 USDC on Base. For the
 *                                  first run also expects EOA nonce = 0 (the path most
 *                                  vulnerable to non-canonical RLP edges); subsequent
 *                                  runs short-circuit via `isDelegated`.
 *   BASE_PIMLICO_BUNDLER_URL     — Pimlico v2 bundler URL for Base
 *                                  (https://api.pimlico.io/v2/8453/rpc?apikey=...)
 *
 * Notes:
 *   - The Alchemy variant reuses `BASE_MAINNET` because Alchemy serves bundler RPC at the
 *     same endpoint as the chain RPC. Override with a dedicated URL if needed.
 *   - Each variant `skipIf`s independently on its bundler URL.
 *   - A successful run spends a small amount of USDC via the Circle paymaster.
 */

const BASE_MAINNET = "EVM-8453"
const BASE_CHAIN_ID = 8453

const RPC_URL = process.env.BASE_MAINNET
const PRIVATE_KEY = process.env.PRIVATE_KEY as HexString | undefined
const PIMLICO_BUNDLER_URL = process.env.BASE_PIMLICO_BUNDLER_URL
// Alchemy's bundler shares its RPC endpoint; reuse BASE_MAINNET unless a separate URL is needed.
const ALCHEMY_BUNDLER_URL = RPC_URL

interface DelegationServicePrivates {
	setupDelegationViaBundler: (chain: string) => Promise<boolean>
	sendBundlerRpc: <T>(bundlerUrl: string, method: string, params: unknown[]) => Promise<T>
}

function build(bundlerUrl: string) {
	const chainConfigs: ResolvedChainConfig[] = [{ chainId: BASE_CHAIN_ID, rpcUrls: [RPC_URL!], bundlerUrl }]
	const configService = new FillerConfigService(chainConfigs)
	const signer = createPrivateKeySigningAccount(PRIVATE_KEY!)
	const clientManager = new ChainClientManager(configService, signer)
	const service = new DelegationService(clientManager, configService, signer)

	// `setupDelegationViaBundler` catches bundler errors and returns false so the caller
	// can fall back to the direct-tx path. For test diagnostics we surface the underlying
	// `eth_sendUserOperation` error instead — a bare `false` would hide why it failed.
	let capturedError: Error | null = null
	const privates = service as unknown as DelegationServicePrivates
	const original = privates.sendBundlerRpc.bind(service)
	privates.sendBundlerRpc = async (url, method, params) => {
		try {
			return await original(url, method, params)
		} catch (err) {
			if (method === "eth_sendUserOperation") {
				capturedError = err as Error
			}
			throw err
		}
	}

	return {
		signer,
		configService,
		clientManager,
		runBundlerDelegation: async () => {
			const success = await privates.setupDelegationViaBundler(BASE_MAINNET)
			if (!success && capturedError) {
				throw capturedError
			}
			return success
		},
	}
}

async function logPreconditions(label: string, ctx: ReturnType<typeof build>) {
	const publicClient = ctx.clientManager.getPublicClient(BASE_MAINNET)
	const address = ctx.signer.account.address
	const nonce = await publicClient.getTransactionCount({ address, blockTag: "latest" })
	const usdcAddress = ctx.configService.getUsdcAsset(BASE_MAINNET)
	const usdc = (await publicClient.readContract({
		address: usdcAddress,
		abi: erc20Abi,
		functionName: "balanceOf",
		args: [address],
	})) as bigint
	// eslint-disable-next-line no-console
	console.log(`[${label}] EOA=${address} nonce=${nonce} usdc=${usdc.toString()} (needs USDC >= 1)`)
}

const skipSuite = !(RPC_URL && PRIVATE_KEY)

describe.skipIf(skipSuite)("DelegationService — Base mainnet EIP-7702 bundler (live integration)", () => {
	it.skipIf(!PIMLICO_BUNDLER_URL)(
		"delegates via Pimlico bundler with Circle paymaster",
		async () => {
			const ctx = build(PIMLICO_BUNDLER_URL!)
			await logPreconditions("pimlico", ctx)
			const success = await ctx.runBundlerDelegation()
			expect(success).toBe(true)
		},
		120_000,
	)

	it.skipIf(!ALCHEMY_BUNDLER_URL)(
		"delegates via Alchemy bundler with Circle paymaster",
		async () => {
			const ctx = build(ALCHEMY_BUNDLER_URL!)
			await logPreconditions("alchemy", ctx)
			const success = await ctx.runBundlerDelegation()
			expect(success).toBe(true)
		},
		120_000,
	)
})
