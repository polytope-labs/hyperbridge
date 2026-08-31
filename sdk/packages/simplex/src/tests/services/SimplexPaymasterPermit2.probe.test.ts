import { describe, it, expect } from "vitest"
import { erc20Abi, parseAbiItem, toEventSelector, type Log } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { DelegationService } from "@/services/DelegationService"
import { UserOpSender } from "@/services/UserOpSender"
import { ChainClientManager } from "@/services/ChainClientManager"
import { FillerConfigService, type ResolvedChainConfig } from "@/services/FillerConfigService"
import { privateKeySigner } from "@/services/wallet/accounts/privatekey"

/**
 * Live probe: does a real bundler accept SimplexPaymaster PERMIT2 mode?
 *
 * PERMIT2 mode writes Permit2's nonce bitmap and the token's Permit2 allowance during
 * paymaster validation — storage that ERC-7562 does not associate with the sender or
 * the paymaster — so a spec-enforcing bundler could reject it. Contract correctness is
 * covered by the fork tests in `evm/`; this suite answers the bundler-policy question
 * against a live deployment (see `evm/script/SimplexPaymasterPermit2Probe.s.sol`).
 *
 * Two ops, both sponsored in mode 0x02 with `skipPermit`:
 *   1. an EIP-7702 delegation of a fresh EOA (Permit2 verifies the ECDSA path, then the
 *      account has code), bootstrapping the one funded approve(Permit2, max) on the way
 *   2. a plain no-op from the now-delegated account (Permit2 takes the ERC-1271 path)
 *
 * Required env (suite skips when missing):
 *   PERMIT2_PROBE_RPC_URL         — Base Sepolia RPC that also serves bundler methods (Alchemy)
 *   PERMIT2_PROBE_PRIVATE_KEY     — funded EOA holding the probe token
 *   PERMIT2_PROBE_PAYMASTER       — SimplexPaymaster proxy with PERMIT2 mode
 *   PERMIT2_PROBE_TOKEN           — registered no-permit token (6 decimals)
 *   PERMIT2_PROBE_SOLVER_ACCOUNT  — SolverAccount to delegate to
 *   PERMIT2_PROBE_BUNDLER_URL     — optional; defaults to the RPC URL
 */

const CHAIN = "EVM-84532"
const CHAIN_ID = 84532
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as HexString

const RPC_URL = process.env.PERMIT2_PROBE_RPC_URL
const PRIVATE_KEY = process.env.PERMIT2_PROBE_PRIVATE_KEY as HexString | undefined
const PAYMASTER = process.env.PERMIT2_PROBE_PAYMASTER as HexString | undefined
const TOKEN = process.env.PERMIT2_PROBE_TOKEN as HexString | undefined
const SOLVER_ACCOUNT = process.env.PERMIT2_PROBE_SOLVER_ACCOUNT as HexString | undefined
const BUNDLER_URL = process.env.PERMIT2_PROBE_BUNDLER_URL ?? RPC_URL

const PERMIT2_EXECUTED_TOPIC = toEventSelector(
	parseAbiItem("event Permit2Executed(address indexed token, address indexed owner, uint256 amount, uint256 nonce)"),
)

function build() {
	const chainConfigs: ResolvedChainConfig[] = [{ chainId: CHAIN_ID, rpcUrls: [RPC_URL!], bundlerUrl: BUNDLER_URL }]
	const configService = new FillerConfigService(chainConfigs)
	// Base Sepolia carries none of these in the SDK registry; point the service at the probe deployment.
	Object.assign(configService, {
		getSimplexPaymasterAddress: () => PAYMASTER,
		getCirclePaymasterAddress: () => undefined,
		getUsdcAsset: () => TOKEN,
		getUsdcDecimals: () => 6,
		getUsdtAsset: () => "0x" as HexString,
		getPermit2Address: () => PERMIT2,
		getSolverAccountContractAddress: () => SOLVER_ACCOUNT,
	})

	const signer = privateKeySigner(PRIVATE_KEY!)
	const clientManager = new ChainClientManager(configService, signer)
	const delegation = new DelegationService(clientManager, configService, signer)
	const userOpSender = new UserOpSender(clientManager, configService, signer)

	// Surface the bundler's rejection reason instead of the service's `false`.
	let capturedError: Error | null = null
	for (const sender of [
		userOpSender,
		(delegation as unknown as { userOpSender: UserOpSender }).userOpSender,
	] as unknown as { sendBundlerRpc: <T>(url: string, method: string, params: unknown[]) => Promise<T> }[]) {
		const original = sender.sendBundlerRpc.bind(sender)
		sender.sendBundlerRpc = async (url, method, params) => {
			try {
				return await original(url, method, params)
			} catch (err) {
				if (method === "eth_sendUserOperation") {
					capturedError = err as Error
					// eslint-disable-next-line no-console
					console.log(`[probe] rejected userOp: ${JSON.stringify(params[0])}`)
				}
				throw err
			}
		}
	}

	return { signer, configService, clientManager, delegation, userOpSender, bundlerError: () => capturedError }
}

async function permit2ExecutedLogs(ctx: ReturnType<typeof build>, txHash: HexString): Promise<Log[]> {
	const receipt = await ctx.clientManager.getPublicClient(CHAIN).getTransactionReceipt({ hash: txHash })
	return receipt.logs.filter(
		(log) => log.address.toLowerCase() === PAYMASTER!.toLowerCase() && log.topics[0] === PERMIT2_EXECUTED_TOPIC,
	)
}

const skipSuite = !(RPC_URL && PRIVATE_KEY && PAYMASTER && TOKEN && SOLVER_ACCOUNT)

describe.skipIf(skipSuite)("SimplexPaymaster PERMIT2 mode — live bundler probe (Base Sepolia)", () => {
	it(
		"sponsors an EIP-7702 delegation in PERMIT2 mode",
		async () => {
			const ctx = build()
			const publicClient = ctx.clientManager.getPublicClient(CHAIN)
			const address = ctx.signer.address
			const balance = (await publicClient.readContract({
				address: TOKEN!,
				abi: erc20Abi,
				functionName: "balanceOf",
				args: [address],
			})) as bigint
			// eslint-disable-next-line no-console
			console.log(`[probe] EOA=${address} token balance=${balance} delegated=${await ctx.delegation.isDelegated(CHAIN)}`)

			const privates = ctx.delegation as unknown as { setupDelegationViaBundler: (chain: string) => Promise<boolean> }
			const success = await privates.setupDelegationViaBundler(CHAIN)
			if (!success && ctx.bundlerError()) throw ctx.bundlerError()
			expect(success).toBe(true)
			expect(await ctx.delegation.isDelegated(CHAIN)).toBe(true)
		},
		180_000,
	)

	it(
		"sponsors a no-op from the delegated account in PERMIT2 mode",
		async () => {
			const ctx = build()
			expect(await ctx.delegation.isDelegated(CHAIN)).toBe(true)

			const result = await ctx.userOpSender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				skipPermit: true,
			})
			if (!result && ctx.bundlerError()) throw ctx.bundlerError()
			expect(result?.txHash).toBeTruthy()

			const logs = await permit2ExecutedLogs(ctx, result!.txHash)
			// eslint-disable-next-line no-console
			console.log(`[probe] tx=${result!.txHash} Permit2Executed logs=${logs.length}`)
			expect(logs.length).toBe(1)
		},
		180_000,
	)
})
