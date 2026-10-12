import { describe, it, expect } from "vitest"
import { createPublicClient, createWalletClient, http, parseEther, type Chain } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"
import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import { IsmpClient } from "@/client"
import { createQueryClient } from "@/queryClient"
import {
	HyperFungibleToken,
	type BridgeParams,
	type BridgeStep,
	type QuoteResult,
} from "@/protocols/hyperFungibleToken"
import EVM_HOST from "@/abis/evmHost"
import { RequestStatus, type HexString } from "@/types"
import { sleep } from "@/utils"

// WrappedHFT wrapping WBNB on BSC testnet (lock/unlock)
const BSC_WRAPPED_HFT = "0x5ae3C15EFa6FC9D226c108bD3c706F2400Ab7311" as const
// HFT on Polygon Amoy (burn/mint), paired with the BSC WrappedHFT
const POLYGON_HFT = "0x1bd0AB7686710a66255d4EFe4826f43CF2A11a1F" as const

const BSC_HOST = "0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1" as const
const POLYGON_HOST = "0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1" as const

// TokenFaucet (drips 1000 fee tokens per day)
const BSC_FAUCET = "0xcb00f5b86aac5e2fdca9dc7f34d9bfe00b967c18" as const

const RECEIPT_POLL_MS = 15_000
const DELIVERED_TIMEOUT_MS = 12 * 60_000
const INDEXER_POLL_MS = 10_000
const INDEXED_TIMEOUT_MS = 5 * 60_000
const DELIVERED_OR_LATER = new Set<string>([
	RequestStatus.HYPERBRIDGE_DELIVERED,
	RequestStatus.HYPERBRIDGE_FINALIZED,
	RequestStatus.DESTINATION,
])
const TIMED_OUT = Symbol("timed out")

const FAUCET_ABI = [
	{
		type: "function",
		name: "drip",
		inputs: [{ name: "token", type: "address" }],
		outputs: [],
		stateMutability: "nonpayable",
	},
] as const

function requireEnv(name: string): string {
	const value = process.env[name]
	if (!value) throw new Error(`${name} is not set; add it to sdk/.env.local`)
	return value
}

function createBscToPolygon() {
	const source = EvmChain.fromParams({
		chainId: 97,
		rpcUrl: requireEnv("BSC_CHAPEL"),
		host: BSC_HOST,
		consensusStateId: "BSC0",
	})

	const dest = EvmChain.fromParams({
		chainId: 80002,
		rpcUrl: requireEnv("POLYGON_AMOY"),
		host: POLYGON_HOST,
		consensusStateId: "POLY",
	})

	return { source, dest }
}

function createPolygonToBsc() {
	const source = EvmChain.fromParams({
		chainId: 80002,
		rpcUrl: requireEnv("POLYGON_AMOY"),
		host: POLYGON_HOST,
		consensusStateId: "POLY",
	})

	const dest = EvmChain.fromParams({
		chainId: 97,
		rpcUrl: requireEnv("BSC_CHAPEL"),
		host: BSC_HOST,
		consensusStateId: "BSC0",
	})

	return { source, dest }
}

async function createIsmpClient(source: EvmChain, dest: EvmChain) {
	const hyperbridge = await SubstrateChain.connect({
		wsUrl: process.env.HYPERBRIDGE_GARGANTUA || "wss://gargantua.rpc.polytope.technology",
		consensusStateId: "PAS0",
		hasher: "Keccak",
		stateMachineId: "KUSAMA-4009",
	})

	const indexerUrl = process.env.GARGANTUA_INDEXER_URL || "https://gargantua.indexer.polytope.technology"
	const queryClient = createQueryClient({ url: indexerUrl })

	const ismpClient = new IsmpClient({
		queryClient,
		source,
		dest,
		hyperbridge,
		pollInterval: 5_000,
	})

	return { ismpClient, hyperbridge, indexerUrl }
}

/**
 * Tops up the account's fee tokens from the TokenFaucet when the balance is below 100.
 * The faucet drips once per day, so a failed drip is logged and ignored.
 */
async function ensureFeeTokens(params: {
	chain: Chain
	rpcUrl: string
	host: HexString
	faucet: HexString
	account: ReturnType<typeof privateKeyToAccount>
}) {
	const { chain, rpcUrl, host, faucet, account } = params
	const publicClient = createPublicClient({ chain, transport: http(rpcUrl) })
	const walletClient = createWalletClient({ account, chain, transport: http(rpcUrl) })

	const feeToken = await publicClient.readContract({
		address: host,
		abi: EVM_HOST.ABI,
		functionName: "feeToken",
	})

	const balance = await publicClient.readContract({
		address: feeToken,
		abi: [
			{
				type: "function",
				name: "balanceOf",
				inputs: [{ name: "account", type: "address" }],
				outputs: [{ type: "uint256" }],
				stateMutability: "view",
			},
		],
		functionName: "balanceOf",
		args: [account.address],
	})
	console.log(`Fee token balance on ${chain.name}: ${balance}`)

	if (balance >= parseEther("100")) return

	try {
		const hash = await walletClient.writeContract({
			address: faucet,
			abi: FAUCET_ABI,
			functionName: "drip",
			args: [feeToken],
		})
		await publicClient.waitForTransactionReceipt({ hash })
		console.log(`Faucet drip confirmed: ${hash}`)
	} catch (e) {
		console.log("Faucet drip skipped:", (e as Error).message)
	}
}

async function withDeadline<T>(promise: Promise<T>, deadline: number): Promise<T | typeof TIMED_OUT> {
	let timer: NodeJS.Timeout | undefined
	const expiry = new Promise<typeof TIMED_OUT>((resolve) => {
		timer = setTimeout(() => resolve(TIMED_OUT), Math.max(0, deadline - Date.now()))
	})
	try {
		return await Promise.race([promise, expiry])
	} finally {
		clearTimeout(timer)
	}
}

/**
 * Sends the bridge transaction on the source chain and returns its commitment once Hyperbridge
 * holds the request. The status stream starts from the indexer's latest status, so a request the
 * indexer first sees already delivered never yields HYPERBRIDGE_DELIVERED; Hyperbridge's request
 * receipt is read directly between stream reads to catch that case.
 */
async function bridgeToHyperbridge(params: {
	hft: HyperFungibleToken
	hyperbridge: SubstrateChain
	token: HexString
	account: ReturnType<typeof privateKeyToAccount>
	chain: Chain
	rpcUrl: string
	dest: string
	amount: bigint
}): Promise<HexString> {
	const { hft, hyperbridge, token, account, chain, rpcUrl, dest, amount } = params
	const walletClient = createWalletClient({ account, chain, transport: http(rpcUrl) })
	const publicClient = createPublicClient({ chain, transport: http(rpcUrl) })

	const gen = hft.bridge({
		token,
		from: account.address,
		to: account.address,
		amount,
		dest,
		timeout: 7200n,
		payInFeeToken: true,
		relayerFee: parseEther("5"),
	})

	try {
		let commitment: HexString | undefined
		let result = await gen.next()
		while (!commitment) {
			if (result.done) throw new Error("The bridge flow ended before the request was submitted")
			const step = result.value

			if (step.type === "approve") {
				const hash = await walletClient.sendTransaction({ to: step.tx.to, data: step.tx.data })
				console.log(`Approve tx: ${hash}`)
				await publicClient.waitForTransactionReceipt({ hash })
				result = await gen.next()
			} else if (step.type === "send") {
				const hash = await walletClient.sendTransaction({
					to: step.tx.to,
					data: step.tx.data,
					value: step.tx.value,
				})
				console.log(`Send tx: ${hash}`)
				result = await gen.next(hash)
			} else if (step.type === "submitted") {
				commitment = step.commitment
				console.log(`Commitment: ${commitment}`)
			} else {
				throw new Error(`Unexpected ${step.status} status before the request was submitted`)
			}
		}

		const deadline = Date.now() + DELIVERED_TIMEOUT_MS
		let pending: Promise<IteratorResult<BridgeStep, void>> | undefined
		while (Date.now() < deadline) {
			if (!pending) {
				pending = gen.next()
				pending.catch(() => {})
			}
			const read = await withDeadline(pending, Math.min(deadline, Date.now() + RECEIPT_POLL_MS))
			if (read !== TIMED_OUT) {
				pending = undefined
				if (read.done)
					throw new Error(`The status stream for ${commitment} ended before Hyperbridge received it`)
				if (read.value.type === "status") {
					console.log(`Status: ${read.value.status}`)
					if (DELIVERED_OR_LATER.has(read.value.status)) return commitment
				}
			}
			if (await hyperbridge.queryRequestReceipt(commitment)) return commitment
		}
		throw new Error(
			`Request ${commitment} did not reach Hyperbridge within ${DELIVERED_TIMEOUT_MS / 60_000} minutes`,
		)
	} finally {
		void gen.return(undefined).catch(() => {})
	}
}

/**
 * Waits until the indexer records the request as delivered to Hyperbridge. The receipt read in
 * bridgeToHyperbridge passes without the indexer, so this is the step that fails when the indexer
 * misses the request.
 */
async function awaitIndexedDelivery(ismpClient: IsmpClient, commitment: HexString, indexerUrl: string): Promise<void> {
	const deadline = Date.now() + INDEXED_TIMEOUT_MS
	let indexed = "unknown"
	let lastError: string | undefined
	while (true) {
		try {
			const request = await ismpClient.queryPostRequest(commitment)
			const statuses = request?.statuses.map(({ status }) => status) ?? []
			if (statuses.some((status) => DELIVERED_OR_LATER.has(status))) return
			indexed = request ? statuses.join(", ") || "none" : "no record"
		} catch (e) {
			lastError = e instanceof Error ? e.message : String(e)
		}
		if (Date.now() >= deadline) {
			throw new Error(
				`The indexer at ${indexerUrl} did not record request ${commitment} as delivered to Hyperbridge within ${INDEXED_TIMEOUT_MS / 60_000} minutes (indexed statuses: ${indexed}${lastError ? `; last query error: ${lastError}` : ""})`,
			)
		}
		await sleep(INDEXER_POLL_MS)
	}
}

describe("HyperFungibleToken SDK", () => {
	describe("isWrapped", () => {
		it("detects WrappedHFT on BSC", async () => {
			const { source, dest } = createBscToPolygon()
			const hft = new HyperFungibleToken({ source, dest })

			expect(await hft.isWrapped(BSC_WRAPPED_HFT)).toBe(true)
		}, 30_000)

		it("detects that HFT on Polygon is not wrapped", async () => {
			const { source, dest } = createPolygonToBsc()
			const hft = new HyperFungibleToken({ source, dest })

			expect(await hft.isWrapped(POLYGON_HFT)).toBe(false)
		}, 30_000)
	})

	it(
		"locks BNB on BSC and delivers the mint request to Hyperbridge",
		async () => {
			const account = privateKeyToAccount(requireEnv("PRIVATE_KEY") as HexString)
			const bscRpc = requireEnv("BSC_CHAPEL")
			const { source, dest } = createBscToPolygon()
			const { ismpClient, hyperbridge, indexerUrl } = await createIsmpClient(source, dest)

			try {
				const hft = new HyperFungibleToken({ source, dest, client: ismpClient })
				// quote() prices the fee through the host's Uniswap router, which is the zero address on testnet.
				hft.quote = async (p: BridgeParams): Promise<QuoteResult> => ({
					totalNativeCost: 0n,
					totalFeeTokenCost: p.relayerFee ?? 0n,
					relayerFeeInFeeToken: p.relayerFee ?? 0n,
				})

				await ensureFeeTokens({
					chain: bscTestnet,
					rpcUrl: bscRpc,
					host: BSC_HOST,
					faucet: BSC_FAUCET,
					account,
				})

				const commitment = await bridgeToHyperbridge({
					hft,
					hyperbridge,
					token: BSC_WRAPPED_HFT,
					account,
					chain: bscTestnet,
					rpcUrl: bscRpc,
					dest: "EVM-80002",
					amount: parseEther("0.001"),
				})

				expect(await hyperbridge.queryRequestReceipt(commitment)).toBeTruthy()
				await awaitIndexedDelivery(ismpClient, commitment, indexerUrl)
			} finally {
				await hyperbridge.disconnect()
			}
		},
		15 * 60 * 1000 + INDEXED_TIMEOUT_MS,
	)
})
