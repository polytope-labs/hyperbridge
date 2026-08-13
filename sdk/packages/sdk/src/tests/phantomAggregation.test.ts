import { concat, encodeFunctionData, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import {
	aggregatePhantomBids,
	extractFillData,
	memoizedSolverBalance,
	orderCommitmentFromDecoded,
	recoverBidSignerViem,
	setAggregationFetch,
	splitBidSignature,
	weightedMedian,
	encodeAcceptedSourceChains,
	AGGREGATION_ATTEMPTS,
	ENTRY_POINT_V08_ADDRESS,
	FILL_ORDER_ABI,
	type FetchLike,
	type HexString,
} from "@/protocols/intents/phantom-aggregation"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { encodeUserOpScale } from "@/chains/intentsCoprocessor"
import type { PackedUserOperation } from "@/types"

const GATEWAY = "0x2d61624A17f361020679FaA16fbB566C344AaF4B"
// USDC and USDT addresses left-padded to bytes32, as they appear in an order's token fields.
const USDC_BYTES32 = "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" as HexString
const USDT_BYTES32 = "0x000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7" as HexString
const DAI_BYTES32 = "0x0000000000000000000000006b175474e89094c44da98b954eedeac495271d0f" as HexString
const SOLVER_AMOUNT = 1_000_000n

// A phantom order as it arrives in a bid: zero output amount (the solver's real quote lives in the
// FillOptions outputs), distinct source and destination.
function phantomOrder() {
	return {
		user: `0x${"00".repeat(32)}`,
		source: "0x6131", // "a1"
		destination: "0x6232", // "b2"
		deadline: 0n,
		nonce: 7n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: USDC_BYTES32, amount: 5_000_000n }],
		output: {
			beneficiary: `0x${"00".repeat(32)}`,
			assets: [{ token: USDT_BYTES32, amount: 0n }],
			call: "0x",
		},
	}
}

function fillOptions() {
	return {
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		outputs: [{ token: USDT_BYTES32, amount: SOLVER_AMOUNT }],
	}
}

// Encodes a fillOrder call wrapped in an ERC-7821 execute batch, the way a solver's bid arrives.
function bidCalldata(target: string = GATEWAY): HexString {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const fillCalldata = (encodeFunctionData as any)({
		abi: FILL_ORDER_ABI,
		functionName: "fillOrder",
		args: [phantomOrder(), fillOptions()],
	}) as HexString
	return encodeERC7821ExecuteBatch([{ target: target as HexString, value: 0n, data: fillCalldata }])
}

// A two pair order where the solver priced the first leg and declined the second by quoting zero.
function multiLegBidCalldata(): HexString {
	const order = phantomOrder()
	order.inputs.push({ token: USDT_BYTES32, amount: 5_000_000n })
	order.output.assets.push({ token: DAI_BYTES32, amount: 0n })

	const options = fillOptions()
	options.outputs.push({ token: DAI_BYTES32, amount: 0n })

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const fillCalldata = (encodeFunctionData as any)({
		abi: FILL_ORDER_ABI,
		functionName: "fillOrder",
		args: [order, options],
	}) as HexString
	return encodeERC7821ExecuteBatch([{ target: GATEWAY, value: 0n, data: fillCalldata }])
}

describe("extractFillData", () => {
	it("decodes the order, output token, and solver amount from a bid's ERC-7821 batch", () => {
		const result = extractFillData(bidCalldata(), GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.legs).toHaveLength(1)
		expect(result!.legs[0].outputToken.toLowerCase()).toBe(USDT_BYTES32.toLowerCase())
		expect(result!.legs[0].solverAmount).toBe(SOLVER_AMOUNT)
		// The decoded order still carries the phantom's zero output amount.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		expect((result!.order as any).output.assets[0].amount).toBe(0n)
	})

	// The pallet bundles every configured pair into one order, so a bid carries a quote per leg and
	// the two lists line up by position.
	it("decodes one leg per asset, pairing each with the amount quoted at the same index", () => {
		const result = extractFillData(multiLegBidCalldata(), GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.legs.map((leg) => leg.outputToken.toLowerCase())).toEqual([
			USDT_BYTES32.toLowerCase(),
			DAI_BYTES32.toLowerCase(),
		])
		expect(result!.legs.map((leg) => leg.solverAmount)).toEqual([SOLVER_AMOUNT, 0n])
	})

	it("returns null when no inner call targets the gateway", () => {
		const other = "0x9999999999999999999999999999999999999999"
		expect(extractFillData(bidCalldata(other), GATEWAY)).toBeNull()
	})

	it("returns null for calldata that is not an ERC-7821 batch", () => {
		expect(extractFillData("0xdeadbeef", GATEWAY)).toBeNull()
	})
})

describe("weightedMedian", () => {
	it("equals the single quote when there is only one", () => {
		expect(weightedMedian([{ price: 100n, weight: 5n }])).toBe(100n)
	})

	it("weights quotes by balance — the high-liquidity solver pulls the median to its price", () => {
		const quotes = [
			{ price: 100n, weight: 1n },
			{ price: 200n, weight: 1n },
			{ price: 300n, weight: 100n },
		]
		// Total weight 102; cumulative reaches half (>=51) only at price 300.
		expect(weightedMedian(quotes)).toBe(300n)
	})

	it("reduces to the lower median when all weights are equal", () => {
		const quotes = [
			{ price: 10n, weight: 7n },
			{ price: 20n, weight: 7n },
			{ price: 30n, weight: 7n },
		]
		expect(weightedMedian(quotes)).toBe(20n)
	})

	it("ignores zero-weight quotes so a solver with no liquidity has no influence", () => {
		const quotes = [
			{ price: 1n, weight: 0n },
			{ price: 500n, weight: 0n },
			{ price: 100n, weight: 10n },
		]
		expect(weightedMedian(quotes)).toBe(100n)
	})

	it("falls back to the unweighted median when every weight is zero", () => {
		const quotes = [
			{ price: 30n, weight: 0n },
			{ price: 10n, weight: 0n },
			{ price: 20n, weight: 0n },
		]
		expect(weightedMedian(quotes)).toBe(20n)
	})

	it("returns the smallest price whose cumulative weight reaches half the total", () => {
		const quotes = [
			{ price: 10n, weight: 3n },
			{ price: 20n, weight: 4n },
			{ price: 30n, weight: 3n },
		]
		// Total 10; cumulative: 3 (10), 7 (20) — 7*2>=10 → median is 20.
		expect(weightedMedian(quotes)).toBe(20n)
	})
})

// ─── bid verification ───────────────────────────────────────────────────────────────────────────

const CHAIN = "EVM-8453"
const CHAIN_ID = 8453n
const SOLVER_ACCOUNT = "0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3"
const SOLVER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const IMPOSTOR_KEY = "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as HexString
// The real commitment of `phantomOrder()`, i.e. keccak256(abi.encode(order)) — the same value
// IntentGatewayV2 derives on-chain. It must be the genuine hash, not an arbitrary constant, because
// a bid is only counted when the order in its calldata hashes to the order being priced.
const COMMITMENT = orderCommitmentFromDecoded(phantomOrder())!
const OTHER_COMMITMENT = `0x${"22".repeat(32)}` as HexString
const SESSION_KEY = phantomOrder().session as HexString
// A bid's nonce key binds it to (order, sessionKey); the top 192 bits of the nonce carry it.
const BID_NONCE = CryptoUtils.bidNonceKey(COMMITMENT, SESSION_KEY) << 64n
const USDT = "0xdac17f958d2ee523a2206206994597c13d831ec7"
const SOLVER_BALANCE = 500_000_000n
const NODE_URL = "http://node.test"

function unsignedUserOp(
	sender: HexString,
	nonce: bigint = BID_NONCE,
	paymasterAndData: HexString = "0x",
): PackedUserOperation {
	return {
		sender,
		nonce,
		initCode: "0x",
		callData: bidCalldata(),
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 50_000n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData,
		signature: "0x",
	}
}

// Builds a bid userOp the way BidManager does: the solver signs the EntryPoint v0.8 userOpHash and
// the order commitment is prepended to the signature.
async function signedBidUserOp(opts: {
	signingKey: HexString
	sender?: HexString
	commitment?: HexString
	nonce?: bigint
	paymasterAndData?: HexString
}): Promise<PackedUserOperation> {
	const signer = privateKeyToAccount(opts.signingKey)
	const userOp = unsignedUserOp(opts.sender ?? (signer.address as HexString), opts.nonce, opts.paymasterAndData)
	const solverSignature = await signer.signTypedData(
		CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
	)
	return { ...userOp, signature: concat([opts.commitment ?? COMMITMENT, solverSignature]) as HexString }
}

// The account a `balanceOf(address)` eth_call is asking about, lowercased.
const balanceOfSubject = (data: string) => `0x${data.slice(-40)}`.toLowerCase()

// Stands in for the Hyperbridge node and the destination chain's RPC: serves the given bids, the
// given account code, and an ERC-20 balance for any eth_call — fixed by default, or per-holder
// when a `balanceFor` is supplied.
function mockRpc(
	bids: PackedUserOperation[],
	codeFor: (account: string) => string,
	balanceFor: (holder: string) => bigint = () => SOLVER_BALANCE,
): FetchLike {
	return async (_url, init) => {
		const payload = JSON.parse(init.body)
		const result =
			payload.method === "intents_getBidsForOrder"
				? bids.map((userOp) => ({
						commitment: COMMITMENT,
						filler: `0x${"ab".repeat(32)}`,
						user_op: encodeUserOpScale(userOp),
					}))
				: payload.method === "eth_getCode"
					? codeFor(payload.params[0])
					: toHex(balanceFor(balanceOfSubject(payload.params[0].data)), { size: 32 })
		return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
	}
}

const delegatedTo = (target: string) => () => `0xef0100${target.slice(2)}`.toLowerCase()

function aggregate(
	bids: PackedUserOperation[],
	codeFor: (account: string) => string,
	balanceFor?: (holder: string) => bigint,
) {
	setAggregationFetch(mockRpc(bids, codeFor, balanceFor))
	return aggregatePhantomBids({
		nodeUrl: NODE_URL,
		evmRpcUrls: { [CHAIN]: "http://base.test" },
		chain: CHAIN,
		gatewayAddress: GATEWAY,
		commitment: COMMITMENT,
		yieldVaults: { [CHAIN]: { [USDT]: [] } },
		solverAccount: SOLVER_ACCOUNT,
	})
}

describe("splitBidSignature", () => {
	it("splits a bid signature into its commitment and 65-byte solver signature", () => {
		const solverSignature = `0x${"cd".repeat(65)}` as HexString
		const result = splitBidSignature(concat([COMMITMENT, solverSignature]) as HexString)

		expect(result).not.toBeNull()
		expect(result!.commitment).toBe(COMMITMENT)
		expect(result!.solverSignature).toBe(solverSignature)
	})

	it("ignores the session signature appended at fill time", () => {
		const solverSignature = `0x${"cd".repeat(65)}` as HexString
		const sessionSignature = `0x${"ef".repeat(65)}` as HexString
		const result = splitBidSignature(concat([COMMITMENT, solverSignature, sessionSignature]) as HexString)

		expect(result!.solverSignature).toBe(solverSignature)
	})

	it("returns null when the signature is too short to hold both parts", () => {
		expect(splitBidSignature(`0x${"cd".repeat(65)}` as HexString)).toBeNull()
		expect(splitBidSignature("0x")).toBeNull()
	})
})

describe("recoverBidSignerViem", () => {
	it("recovers the solver that signed the userOp hash", async () => {
		const signer = privateKeyToAccount(SOLVER_KEY)
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		const { solverSignature } = splitBidSignature(userOp.signature)!

		const recovered = await recoverBidSignerViem(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, solverSignature)

		expect(recovered!.toLowerCase()).toBe(signer.address.toLowerCase())
	})

	it("does not recover the solver once the signed operation is tampered with", async () => {
		const signer = privateKeyToAccount(SOLVER_KEY)
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		const { solverSignature } = splitBidSignature(userOp.signature)!

		const recovered = await recoverBidSignerViem(
			{ ...userOp, callData: bidCalldata("0x9999999999999999999999999999999999999999") },
			ENTRY_POINT_V08_ADDRESS,
			CHAIN_ID,
			solverSignature,
		)

		expect(recovered!.toLowerCase()).not.toBe(signer.address.toLowerCase())
	})

	it("returns null for a signature it cannot recover from", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		expect(await recoverBidSignerViem(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID, "0xdeadbeef")).toBeNull()
	})
})

describe("aggregatePhantomBids bid verification", () => {
	it("counts a bid whose sender signed it and is delegated to the chain's SolverAccount", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		const result = await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))

		expect(result).not.toBeNull()
		expect(result!.legs).toHaveLength(1)
		expect(result!.legs[0].legIndex).toBe(0)
		expect(result!.legs[0].bidCount).toBe(1)
		expect(result!.legs[0].medianPrice).toBe(SOLVER_AMOUNT)
	})

	it("drops a bid whose sender is a plain EOA with no delegation", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		expect(await aggregate([userOp], () => "0x")).toBeNull()
	})

	it("drops a bid whose sender is delegated to some other contract", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		expect(await aggregate([userOp], delegatedTo("0x9999999999999999999999999999999999999999"))).toBeNull()
	})

	it("drops a bid signed by someone other than its sender", async () => {
		// A delegated solver's address on an operation signed by a key that does not control it.
		const userOp = await signedBidUserOp({
			signingKey: IMPOSTOR_KEY,
			sender: privateKeyToAccount(SOLVER_KEY).address as HexString,
		})

		expect(await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))).toBeNull()
	})

	it("drops a bid whose signature is bound to a different order", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY, commitment: OTHER_COMMITMENT })

		expect(await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))).toBeNull()
	})

	// A bid's signature covers the userOpHash, which EXCLUDES userOp.signature — so the 32-byte
	// commitment prefix is attacker-mutable. Binding must come from the signed nonce key instead,
	// otherwise a solver's bid for order A can be replayed into order B by rewriting the prefix.
	it("drops a bid replayed into another order by rewriting the unsigned signature prefix", async () => {
		// Signed for OTHER_COMMITMENT (so its nonce binds to that order), then the prefix is swapped
		// to the order being priced. The signature stays valid — only the nonce check catches this.
		const otherNonce = CryptoUtils.bidNonceKey(OTHER_COMMITMENT, SESSION_KEY) << 64n
		const victim = await signedBidUserOp({ signingKey: SOLVER_KEY, nonce: otherNonce })
		const replayed = { ...victim, signature: concat([COMMITMENT, `0x${victim.signature.slice(66)}` as HexString]) }

		expect(await aggregate([replayed as PackedUserOperation], delegatedTo(SOLVER_ACCOUNT))).toBeNull()
	})

	// Bids are stored per substrate filler, so one solver's bid can be resubmitted under many
	// fillers. Weight belongs to the EVM solver, so it must only count once.
	it("counts a solver once even when its bid is duplicated across fillers", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		const result = await aggregate([userOp, userOp, userOp], delegatedTo(SOLVER_ACCOUNT))

		expect(result).not.toBeNull()
		expect(result!.legs[0].bidCount).toBe(1)
		expect(result!.lpBalances.map((lp) => lp.solver.toLowerCase())).toEqual([
			privateKeyToAccount(SOLVER_KEY).address.toLowerCase(),
		])
	})

	it("drops a bid whose calldata order is not the order being priced", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		setAggregationFetch(mockRpc([userOp], delegatedTo(SOLVER_ACCOUNT)))

		// Same bid, but aggregated for a different order than the one its calldata describes.
		const result = await aggregatePhantomBids({
			nodeUrl: NODE_URL,
			evmRpcUrls: { [CHAIN]: "http://base.test" },
			chain: CHAIN,
			gatewayAddress: GATEWAY,
			commitment: OTHER_COMMITMENT,
			yieldVaults: { [CHAIN]: { [USDT]: [] } },
			solverAccount: SOLVER_ACCOUNT,
		})

		expect(result).toBeNull()
	})

	it("prices only the verified bids when unverified ones are mixed in", async () => {
		const solver = await signedBidUserOp({ signingKey: SOLVER_KEY })
		const impostor = await signedBidUserOp({ signingKey: IMPOSTOR_KEY })
		const impostorAddress = privateKeyToAccount(IMPOSTOR_KEY).address.toLowerCase()

		const result = await aggregate([solver, impostor], (account) =>
			account.toLowerCase() === impostorAddress ? "0x" : delegatedTo(SOLVER_ACCOUNT)(),
		)

		expect(result!.legs[0].bidCount).toBe(1)
		// Liquidity is only swept for solvers whose bid was counted.
		expect(result!.lpBalances.map((lp) => lp.solver.toLowerCase())).toEqual([
			privateKeyToAccount(SOLVER_KEY).address.toLowerCase(),
		])
	})

	it("reports each leg's bidders with their inventory weight and no declaration as null", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		const result = await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))

		expect(result!.legs[0].bidders).toEqual([
			{
				solver: privateKeyToAccount(SOLVER_KEY).address.toLowerCase(),
				weight: SOLVER_BALANCE,
				acceptedSources: null,
			},
		])
	})

	// paymasterAndData is covered by the userOpHash, so the declaration carries the same
	// authenticity as the quote — and signing over it keeps these fixtures' signatures valid.
	it("decodes the accepted-source-chains declaration out of a bid's paymasterAndData", async () => {
		const declared = ["EVM-1", "EVM-42161"]
		const userOp = await signedBidUserOp({
			signingKey: SOLVER_KEY,
			paymasterAndData: encodeAcceptedSourceChains(declared),
		})

		const result = await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))

		expect(result!.legs[0].bidders[0].acceptedSources).toEqual(declared)
	})

	it("keeps an explicit empty declaration distinct from an absent one", async () => {
		const userOp = await signedBidUserOp({
			signingKey: SOLVER_KEY,
			paymasterAndData: encodeAcceptedSourceChains([]),
		})

		const result = await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT))

		expect(result!.legs[0].bidders[0].acceptedSources).toEqual([])
	})

	// The weight IS the solver's output-token inventory on the destination chain. When every quote
	// for a leg carries zero weight there is nothing to weight the median by, and weightedMedian
	// can only pick by position — so on an even-sized set the highest quote wins and any solver
	// holding nothing sets the published rate for free. Such legs are dropped, not priced.
	it("drops a leg when no bidder holds the output token on the destination chain", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })

		const result = await aggregate([userOp], delegatedTo(SOLVER_ACCOUNT), () => 0n)

		expect(result).not.toBeNull()
		expect(result!.legs).toEqual([])
	})

	// A zero-inventory co-bidder is excluded outright, not merely down-weighted: counting it would
	// overstate how many solvers stand behind the price, and carrying it into `bidders` would mint
	// a zero-capacity PoolBidder row and, through its declaration, a PoolRoute advertising a
	// corridor nobody can actually fill.
	it("keeps a leg backed by one solver but excludes its zero-inventory co-bidder", async () => {
		const backed = privateKeyToAccount(SOLVER_KEY).address.toLowerCase()
		const solver = await signedBidUserOp({ signingKey: SOLVER_KEY })
		const empty = await signedBidUserOp({ signingKey: IMPOSTOR_KEY })

		const result = await aggregate([solver, empty], delegatedTo(SOLVER_ACCOUNT), (holder) =>
			holder === backed ? SOLVER_BALANCE : 0n,
		)

		expect(result!.legs).toHaveLength(1)
		expect(result!.legs[0].bidCount).toBe(1)
		expect(result!.legs[0].bidders.map((b) => b.solver.toLowerCase())).toEqual([backed])
		expect(result!.legs[0].bidders.map((b) => b.weight)).toEqual([SOLVER_BALANCE])
	})

	// Production incident: a throttled destination-chain RPC answered eth_getCode with a body that
	// had no `result`, the delegation check read that as "not a delegated solver", and the bid was
	// dropped. The four legs only that solver quoted vanished and its pool reported zero depth
	// against ~60M cNGN it actually held. An unreadable node must never look like a verdict.
	it("gives up rather than dropping a bid whose delegation could not be read", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		let getCodeCalls = 0
		setAggregationFetch(async (_url, init) => {
			const payload = JSON.parse(init.body)
			if (payload.method === "eth_getCode") {
				getCodeCalls += 1
				// A throttle response: HTTP 200, valid JSON, no `result`.
				return { json: async () => ({ id: payload.id, jsonrpc: "2.0", error: { code: -32005 } }) }
			}
			const result =
				payload.method === "intents_getBidsForOrder"
					? [{ commitment: COMMITMENT, filler: `0x${"ab".repeat(32)}`, user_op: encodeUserOpScale(userOp) }]
					: toHex(SOLVER_BALANCE, { size: 32 })
			return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
		})

		await expect(
			aggregatePhantomBids({
				nodeUrl: NODE_URL,
				evmRpcUrls: { [CHAIN]: "http://base.test" },
				chain: CHAIN,
				gatewayAddress: GATEWAY,
				commitment: COMMITMENT,
				yieldVaults: { [CHAIN]: { [USDT]: [] } },
				solverAccount: SOLVER_ACCOUNT,
			}),
		).rejects.toThrow()
		// Every attempt re-read; the failure was never cached as a verdict.
		expect(getCodeCalls).toBeGreaterThanOrEqual(AGGREGATION_ATTEMPTS)
	}, 30_000)

	// The dedupe that collapses one solver's bid across N fillers runs AFTER verification, so
	// without a cache every copy re-asked the chain the same question — and every retry re-asked
	// it for every solver, hammering the endpoint most likely to be throttled.
	it("reads a solver's delegation once per aggregation, not once per duplicate bid", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		let getCodeCalls = 0
		setAggregationFetch(async (_url, init) => {
			const payload = JSON.parse(init.body)
			if (payload.method === "eth_getCode") getCodeCalls += 1
			const result =
				payload.method === "intents_getBidsForOrder"
					? [userOp, userOp, userOp].map((op) => ({
							commitment: COMMITMENT,
							filler: `0x${"ab".repeat(32)}`,
							user_op: encodeUserOpScale(op),
						}))
					: payload.method === "eth_getCode"
						? delegatedTo(SOLVER_ACCOUNT)()
						: toHex(SOLVER_BALANCE, { size: 32 })
			return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
		})

		const result = await aggregatePhantomBids({
			nodeUrl: NODE_URL,
			evmRpcUrls: { [CHAIN]: "http://base.test" },
			chain: CHAIN,
			gatewayAddress: GATEWAY,
			commitment: COMMITMENT,
			yieldVaults: { [CHAIN]: { [USDT]: [] } },
			solverAccount: SOLVER_ACCOUNT,
		})

		expect(result!.legs[0].bidCount).toBe(1)
		expect(getCodeCalls).toBe(1)
	})

	it("does not re-read a verified solver's delegation when a retry is forced elsewhere", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		let getCodeCalls = 0
		let runs = 0
		setAggregationFetch(async (_url, init) => {
			const payload = JSON.parse(init.body)
			if (payload.method === "intents_getBidsForOrder") runs += 1
			if (payload.method === "eth_getCode") getCodeCalls += 1
			// The first run dies on a balance read, long after delegation was settled.
			if (payload.method === "eth_call" && runs < 2) {
				return { json: async () => ({ id: payload.id, jsonrpc: "2.0", error: { code: -32005 } }) }
			}
			const result =
				payload.method === "intents_getBidsForOrder"
					? [{ commitment: COMMITMENT, filler: `0x${"ab".repeat(32)}`, user_op: encodeUserOpScale(userOp) }]
					: payload.method === "eth_getCode"
						? delegatedTo(SOLVER_ACCOUNT)()
						: toHex(SOLVER_BALANCE, { size: 32 })
			return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
		})

		const result = await aggregatePhantomBids({
			nodeUrl: NODE_URL,
			evmRpcUrls: { [CHAIN]: "http://base.test" },
			chain: CHAIN,
			gatewayAddress: GATEWAY,
			commitment: COMMITMENT,
			yieldVaults: { [CHAIN]: { [USDT]: [] } },
			solverAccount: SOLVER_ACCOUNT,
		})

		expect(runs).toBeGreaterThan(1)
		expect(result!.legs[0].bidders[0].weight).toBe(SOLVER_BALANCE)
		// Verified on the first run and still trusted on the second.
		expect(getCodeCalls).toBe(1)
	}, 30_000)

	it("retries the whole run and succeeds once the RPC recovers", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		let attempts = 0
		setAggregationFetch(async (_url, init) => {
			const payload = JSON.parse(init.body)
			if (payload.method === "intents_getBidsForOrder") attempts += 1
			// First run's balance reads fail outright; later runs are healthy.
			if (payload.method === "eth_call" && attempts < 2) {
				return { json: async () => ({ id: payload.id, jsonrpc: "2.0", error: { code: -32005 } }) }
			}
			const result =
				payload.method === "intents_getBidsForOrder"
					? [{ commitment: COMMITMENT, filler: `0x${"ab".repeat(32)}`, user_op: encodeUserOpScale(userOp) }]
					: payload.method === "eth_getCode"
						? delegatedTo(SOLVER_ACCOUNT)()
						: toHex(SOLVER_BALANCE, { size: 32 })
			return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
		})

		const result = await aggregatePhantomBids({
			nodeUrl: NODE_URL,
			evmRpcUrls: { [CHAIN]: "http://base.test" },
			chain: CHAIN,
			gatewayAddress: GATEWAY,
			commitment: COMMITMENT,
			yieldVaults: { [CHAIN]: { [USDT]: [] } },
			solverAccount: SOLVER_ACCOUNT,
			// A shared memo must not carry the first run's failure into the retry.
			getBalance: memoizedSolverBalance({ [CHAIN]: { [USDT]: [] } }),
		})

		expect(attempts).toBeGreaterThan(1)
		expect(result!.legs[0].bidders[0].weight).toBe(SOLVER_BALANCE)
	}, 30_000)

	// One bundled order per configured chain means several aggregations run against the same
	// block; a caller-supplied reader lets them share one point-in-time balance cache.
	it("serves repeat aggregations from a shared balance reader without re-reading balances", async () => {
		const userOp = await signedBidUserOp({ signingKey: SOLVER_KEY })
		let balanceCalls = 0
		const rpc = mockRpc([userOp], delegatedTo(SOLVER_ACCOUNT))
		setAggregationFetch(async (url, init) => {
			const method = JSON.parse((init as { body: string }).body).method
			if (method !== "intents_getBidsForOrder" && method !== "eth_getCode") balanceCalls++
			return rpc(url, init)
		})

		const yieldVaults = { [CHAIN]: { [USDT]: [] } }
		const params = {
			nodeUrl: NODE_URL,
			evmRpcUrls: { [CHAIN]: "http://base.test" },
			chain: CHAIN,
			gatewayAddress: GATEWAY,
			commitment: COMMITMENT,
			yieldVaults,
			solverAccount: SOLVER_ACCOUNT,
			getBalance: memoizedSolverBalance(yieldVaults),
		}
		expect(await aggregatePhantomBids(params)).not.toBeNull()
		const afterFirst = balanceCalls
		expect(afterFirst).toBeGreaterThan(0)

		expect(await aggregatePhantomBids(params)).not.toBeNull()
		expect(balanceCalls).toBe(afterFirst)
	})
})
