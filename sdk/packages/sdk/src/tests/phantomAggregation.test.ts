import { concat, encodeFunctionData, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import {
	aggregatePhantomBids,
	extractFillData,
	orderCommitmentFromDecoded,
	recoverBidSignerViem,
	setAggregationFetch,
	splitBidSignature,
	weightedMedian,
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

describe("extractFillData", () => {
	it("decodes the order, output token, and solver amount from a bid's ERC-7821 batch", () => {
		const result = extractFillData(bidCalldata(), GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.outputToken.toLowerCase()).toBe(USDT_BYTES32.toLowerCase())
		expect(result!.solverAmount).toBe(SOLVER_AMOUNT)
		// The decoded order still carries the phantom's zero output amount.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		expect((result!.order as any).output.assets[0].amount).toBe(0n)
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

function unsignedUserOp(sender: HexString, nonce: bigint = BID_NONCE): PackedUserOperation {
	return {
		sender,
		nonce,
		initCode: "0x",
		callData: bidCalldata(),
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 50_000n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
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
}): Promise<PackedUserOperation> {
	const signer = privateKeyToAccount(opts.signingKey)
	const userOp = unsignedUserOp(opts.sender ?? (signer.address as HexString), opts.nonce)
	const solverSignature = await signer.signTypedData(
		CryptoUtils.packedUserOpTypedData(userOp, ENTRY_POINT_V08_ADDRESS, CHAIN_ID),
	)
	return { ...userOp, signature: concat([opts.commitment ?? COMMITMENT, solverSignature]) as HexString }
}

// Stands in for the Hyperbridge node and the destination chain's RPC: serves the given bids, the
// given account code, and a fixed ERC-20 balance for any eth_call.
function mockRpc(bids: PackedUserOperation[], codeFor: (account: string) => string): FetchLike {
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
					: toHex(SOLVER_BALANCE, { size: 32 })
		return { json: async () => ({ id: payload.id, jsonrpc: "2.0", result }) }
	}
}

const delegatedTo = (target: string) => () => `0xef0100${target.slice(2)}`.toLowerCase()

function aggregate(bids: PackedUserOperation[], codeFor: (account: string) => string) {
	setAggregationFetch(mockRpc(bids, codeFor))
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
		expect(result!.bidCount).toBe(1)
		expect(result!.medianPrice).toBe(SOLVER_AMOUNT)
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
		expect(result!.bidCount).toBe(1)
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

		expect(result!.bidCount).toBe(1)
		// Liquidity is only swept for solvers whose bid was counted.
		expect(result!.lpBalances.map((lp) => lp.solver.toLowerCase())).toEqual([
			privateKeyToAccount(SOLVER_KEY).address.toLowerCase(),
		])
	})
})
