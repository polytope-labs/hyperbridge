// Places one scenario's order as a user and executes it with the SDK until it is filled.
// Usage: node e2e/swap.mjs <scenario> <resultFile>
// Settlement through Hyperbridge is not awaited: a fill on the destination chain is the end.
import fs from "node:fs"
import { createPublicClient, createWalletClient, erc20Abi, formatUnits, http, keccak256, maxUint256, parseUnits, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { DEFAULT_GRAFFITI, EvmChain, IntentGateway, IntentsCoprocessor, bytes20ToBytes32 } from "@hyperbridge/sdk"
import { FEE_TOKEN, GATEWAY, HOST, SCENARIOS, TOKENS, chains, readEnv } from "./env.mjs"

const [scenario, resultFile] = process.argv.slice(2)
const TIMEOUT_MS = Number(process.env.E2E_SCENARIO_TIMEOUT_MIN || 10) * 60_000

const json = (value) => JSON.stringify(value, (_k, v) => (typeof v === "bigint" ? v.toString() : v))
const log = (event, data = {}) => console.log(json({ at: new Date().toISOString(), scenario, event, ...data }))

async function ensureAllowance(pub, wallet, token) {
	const owner = wallet.account.address
	const allowance = await pub.readContract({ address: token, abi: erc20Abi, functionName: "allowance", args: [owner, GATEWAY] })
	if (allowance > parseUnits("1000000000", 18)) return
	const hash = await wallet.writeContract({ address: token, abi: erc20Abi, functionName: "approve", args: [GATEWAY, maxUint256] })
	await pub.waitForTransactionReceipt({ hash })
	log("approve", { token, hash })
}

async function main() {
	const s = SCENARIOS[scenario]
	if (!s) throw new Error(`Unknown scenario ${scenario}`)
	const env = readEnv()
	const CHAINS = chains(env)
	const user = privateKeyToAccount([env.user1Key, env.user2Key][s.user])
	const src = CHAINS[s.source]
	const dst = CHAINS[s.dest]
	log("start", { user: user.address, source: s.source, dest: s.dest, legs: s.legs })

	const srcPub = createPublicClient({ chain: src.viem, transport: http(src.rpc) })
	const srcWallet = createWalletClient({ account: user, chain: src.viem, transport: http(src.rpc) })
	for (const leg of s.legs) await ensureAllowance(srcPub, srcWallet, TOKENS[s.source][leg.tokenIn].address)
	await ensureAllowance(srcPub, srcWallet, FEE_TOKEN)

	const sourceChain = EvmChain.fromParams({ chainId: src.id, host: HOST, rpcUrl: src.rpc, bundlerUrl: src.bundler })
	const destChain =
		s.source === s.dest ? sourceChain : EvmChain.fromParams({ chainId: dst.id, host: HOST, rpcUrl: dst.rpc, bundlerUrl: dst.bundler })
	const coprocessor = await IntentsCoprocessor.connect(env.hyperbridge)
	const gateway = await IntentGateway.create(sourceChain, destChain, coprocessor)

	const beneficiary = bytes20ToBytes32(user.address)
	const order = {
		user: beneficiary,
		source: toHex(s.source),
		destination: toHex(s.dest),
		deadline: 12545151568145n,
		nonce: BigInt(Date.now()),
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: s.legs.map((leg) => {
			const token = TOKENS[s.source][leg.tokenIn]
			return { token: bytes20ToBytes32(token.address), amount: parseUnits(leg.amountIn, token.decimals) }
		}),
		output: {
			beneficiary,
			assets: s.legs.map((leg) => {
				const token = TOKENS[s.dest][leg.tokenOut]
				return { token: bytes20ToBytes32(token.address), amount: parseUnits(leg.minOut, token.decimals) }
			}),
			call: "0x",
		},
	}

	const result = { scenario, user: user.address, outcome: undefined, fills: [], bidRounds: [] }
	const deadline = Date.now() + TIMEOUT_MS
	const stream = gateway.executeBest(order, DEFAULT_GRAFFITI, { auctionTimeMs: 45_000, pollIntervalMs: 5_000 })
	let step = await stream.next()
	try {
		while (!step.done && Date.now() < deadline) {
			const update = step.value
			if (update?.status === "AWAITING_PLACE_ORDER") {
				const request = await srcPub.prepareTransactionRequest({
					to: update.to,
					data: update.data,
					value: update.value ?? 0n,
					account: user,
					chain: src.viem,
				})
				const signed = await srcWallet.signTransaction(request)
				result.placementTx = keccak256(signed)
				log("placement-tx", { hash: result.placementTx })
				step = await stream.next(signed)
				continue
			}
			log(update?.status ?? "update", {
				commitment: update?.commitment,
				transactionHash: update?.transactionHash,
				selectedSolver: update?.selectedSolver,
				filledAssets: update?.filledAssets ?? update?.totalFilledAssets,
				remainingAssets: update?.remainingAssets,
				error: update?.error,
			})
			if (update?.status === "ORDER_PLACED") result.commitment = update.commitment
			if (update?.status === "BIDS_RECEIVED") {
				result.bidRounds.push(
					update.bids.map((bid) => ({ solver: bid.solverAddress, outputs: bid.outputs.map((output) => output.amount) })),
				)
			}
			if (update?.status === "PARTIAL_FILL" || update?.status === "FILLED") {
				result.fills.push({ solver: update.selectedSolver, transactionHash: update.transactionHash })
			}
			if (["FILLED", "EXPIRED", "FAILED", "CANCELLED"].includes(update?.status)) {
				result.outcome = update.status
				break
			}
			step = await stream.next()
		}
	} finally {
		// Not awaited: the executor's teardown can outlive a finished order.
		void stream.return(undefined).catch(() => {})
	}
	result.outcome ??= Date.now() >= deadline ? "TIMEOUT" : "ENDED"
	const received = {}
	const dstPub = createPublicClient({ chain: dst.viem, transport: http(dst.rpc) })
	for (const symbol of new Set(s.legs.map((leg) => leg.tokenOut))) {
		const token = TOKENS[s.dest][symbol]
		const balance = await dstPub.readContract({ address: token.address, abi: erc20Abi, functionName: "balanceOf", args: [user.address] })
		received[`${s.dest}:${symbol}`] = formatUnits(balance, token.decimals)
	}
	result.balancesAfter = received
	log("result", result)
	fs.writeFileSync(resultFile, json(result))
	await coprocessor.disconnect().catch(() => {})
	process.exit(0)
}

main().catch((error) => {
	log("error", { message: String(error?.message ?? error) })
	fs.writeFileSync(resultFile, json({ scenario, outcome: "ERROR", error: String(error?.message ?? error), fills: [] }))
	process.exit(1)
})
