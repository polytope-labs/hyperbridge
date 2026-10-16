import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { toHex } from "viem"
import { CryptoUtils, rundlerUserOperationFees, type HexString } from "@hyperbridge/sdk"

import { UserOpSender, type Eip7702Authorization, type SponsoredUserOpRequest } from "@/services/UserOpSender"
import { SubmittedUnconfirmedError } from "@/services/submission"
import { buildPaymasterAndData } from "@/services/paymaster"
import { packPaymasterAndData, VERIFICATION_GAS_LIMIT_PERMIT2, POST_OP_GAS_LIMIT_SIMPLEX } from "@/services/paymaster/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Regression tests for the bootstrap-approve nonce race: building Simplex
 * paymaster data can send an on-chain `approve(Permit2, max)` from the authority
 * EOA. An EIP-7702 authorization signed *before* that tx embeds the pre-approve nonce,
 * so the bundler rejects the op ("EIP-7702 nonce mismatch"). The sender must
 * therefore resolve the authorization factory only after paymaster data is
 * built — and treat any preparation failure as "never submitted" (null).
 */

vi.mock("@/services/paymaster", () => ({
	hasPaymaster: () => true,
	buildPaymasterAndData: vi.fn(),
}))

vi.mock("@hyperbridge/sdk", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@hyperbridge/sdk")>()
	return { ...actual, rundlerUserOperationFees: vi.fn(actual.rundlerUserOperationFees) }
})

const CHAIN = "EVM-56"
const CHAIN_ID = 56
const ENTRY_POINT = "0x4337843378433784337843378433784337843378" as HexString
const PAYMASTER = "0x00000000000000000000000000000000000000aa" as HexString
const TOKEN = "0x00000000000000000000000000000000000000bb" as HexString
const DELEGATE = "0x00000000000000000000000000000000000000cc" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString

// Fixed limits as passed by the delegation flow — skips bundler gas estimation.
const GAS = { verificationGasLimit: 150_000n, callGasLimit: 50_000n, preVerificationGas: 100_000n }

const configServiceFor = (bundlerUrl: string) =>
	({
		getEntryPointAddress: () => ENTRY_POINT,
		getBundlerUrl: () => bundlerUrl,
		getChainId: () => CHAIN_ID,
	}) as unknown as FillerConfigService

const configService = configServiceFor("http://127.0.0.1:1/bundler")

const publicClient = {
	getGasPrice: async () => 1_000_000_000n,
	getBlock: async () => ({ baseFeePerGas: 900_000_000n }),
	// Only EntryPoint.getNonce is read on this path (fixed gas limits skip estimation).
	readContract: async () => 0n,
}

const clientManager = {
	getPublicClient: () => publicClient,
	getWalletClient: () => ({}),
} as unknown as ChainClientManager

const signer = {
	address: SOLVER,
	signTypedData: async () => ("0x" + "11".repeat(65)) as HexString,
} as unknown as Signer

const permit2ModePaymasterAndData = packPaymasterAndData({
	paymaster: PAYMASTER,
	paymasterData: "0x02" as HexString,
	paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT2,
	paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
})

let bundlerCalls: Array<{ method: string; params: unknown[]; signal?: AbortSignal }>
let bundlerReplies: Record<string, { result?: unknown; error?: unknown }>

beforeEach(() => {
	bundlerCalls = []
	bundlerReplies = {}
	vi.mocked(buildPaymasterAndData).mockReset()
	vi.mocked(rundlerUserOperationFees).mockClear()
	vi.stubGlobal("fetch", async (_url: unknown, init?: { body?: string; signal?: AbortSignal }) => {
		const { method, params } = JSON.parse(init?.body ?? "{}") as { method: string; params: unknown[] }
		bundlerCalls.push({ method, params, signal: init?.signal })
		const reply = bundlerReplies[method]
		if (reply) return { json: async () => ({ jsonrpc: "2.0", id: 1, ...reply }) }
		let result: unknown
		switch (method) {
			case "eth_sendUserOperation":
				result = ("0x" + "ab".repeat(32)) as HexString
				break
			case "eth_getUserOperationReceipt":
				result = { success: true, receipt: { transactionHash: ("0x" + "cd".repeat(32)) as HexString } }
				break
			default:
				return { json: async () => ({ jsonrpc: "2.0", id: 1, error: { message: `no ${method}` } }) }
		}
		return { json: async () => ({ jsonrpc: "2.0", id: 1, result }) }
	})
})

afterEach(() => {
	vi.unstubAllGlobals()
})

describe("UserOpSender EIP-7702 authorization ordering", () => {
	it("signs the authorization after paymaster data is built, so the approve tx nonce is reflected", async () => {
		// Simulates the BSC incident: the EOA starts at nonce 0, and building
		// the Permit2 bootstrap approve mines a tx that bumps it to 1.
		let eoaNonce = 0
		vi.mocked(buildPaymasterAndData).mockImplementation(async () => {
			eoaNonce += 1
			return { paymasterAndData: permit2ModePaymasterAndData, type: "simplex", address: PAYMASTER, token: TOKEN }
		})

		// Mirrors DelegationService.buildAuthorization: reads the nonce at signing time.
		const signAuthorization = vi.fn(
			async (): Promise<Eip7702Authorization> => ({
				chainId: CHAIN_ID,
				address: DELEGATE,
				nonce: eoaNonce,
				r: ("0x" + "22".repeat(32)) as HexString,
				s: ("0x" + "33".repeat(32)) as HexString,
				yParity: 0,
			}),
		)

		const sender = new UserOpSender(clientManager, configService, signer)
		const result = await sender.trySendSponsored({
			chain: CHAIN,
			callData: "0x" as HexString,
			eip7702Auth: signAuthorization,
			gas: GAS,
		})

		expect(result).toEqual({ txHash: "0x" + "cd".repeat(32) })
		expect(signAuthorization).toHaveBeenCalledOnce()

		// The authorization submitted to the bundler must carry the post-approve
		// nonce (1) — a pre-signed tuple would carry the stale 0 and be rejected.
		const send = bundlerCalls.find((c) => c.method === "eth_sendUserOperation")
		expect(send).toBeDefined()
		const [op] = send!.params as [{ sender?: string; eip7702Auth?: { nonce: string } }]
		// The op must be attributed to the signer's address — this is what the
		// signer-interface migration changed (`signer.address`, was
		// `signer.account.address`), and a stale stub let it go out undefined.
		expect(op.sender).toBe(SOLVER)
		expect(op.eip7702Auth?.nonce).toBe(toHex(1))
	})

	it("returns null without signing an authorization when paymaster preparation fails", async () => {
		vi.mocked(buildPaymasterAndData).mockRejectedValue(
			new Error("SimplexPaymaster needs a one-time funded approval on this chain"),
		)
		const signAuthorization = vi.fn(async (): Promise<Eip7702Authorization> => {
			throw new Error("should not be called")
		})

		const sender = new UserOpSender(clientManager, configService, signer)
		await expect(
			sender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				eip7702Auth: signAuthorization,
				gas: GAS,
			}),
		).resolves.toBeNull()

		expect(signAuthorization).not.toHaveBeenCalled()
		expect(bundlerCalls.some((c) => c.method === "eth_sendUserOperation")).toBe(false)
	})

	it("returns null without submitting when the authorization factory throws", async () => {
		// The delegation flow refuses to sign a bundler authorization while an EOA tx is in
		// flight, and the fallback to native depends on that throw reading as "never
		// submitted" (null) rather than an ambiguous submission.
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		const sender = new UserOpSender(clientManager, configService, signer)

		await expect(
			sender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				eip7702Auth: async () => {
					throw new Error(
						`The wallet has a transaction in flight on ${CHAIN}; the delegation is signed once it has landed`,
					)
				},
				gas: GAS,
			}),
		).resolves.toBeNull()

		expect(bundlerCalls.some((c) => c.method === "eth_sendUserOperation")).toBe(false)
	})

	it("returns null without signing an authorization when no paymaster is available", async () => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: "0x" as HexString,
			type: "none",
			reason: "insufficient stablecoin balance for all configured paymasters",
		})
		const signAuthorization = vi.fn(async (): Promise<Eip7702Authorization> => {
			throw new Error("should not be called")
		})

		const sender = new UserOpSender(clientManager, configService, signer)
		await expect(
			sender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				eip7702Auth: signAuthorization,
				gas: GAS,
			}),
		).resolves.toBeNull()

		expect(signAuthorization).not.toHaveBeenCalled()
		expect(bundlerCalls.some((c) => c.method === "eth_sendUserOperation")).toBe(false)
	})
})

describe("UserOpSender gas price", () => {
	const sponsorWith = async (bundlerUrl: string) => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		const sender = new UserOpSender(clientManager, configServiceFor(bundlerUrl), signer)
		return sender.trySendSponsored({ chain: CHAIN, callData: "0x" as HexString, gas: GAS })
	}

	const sentFees = () =>
		bundlerCalls
			.filter((c) => c.method === "eth_sendUserOperation")
			.map((c) => {
				const [op] = c.params as [{ maxFeePerGas: HexString; maxPriorityFeePerGas: HexString }]
				return { maxFeePerGas: BigInt(op.maxFeePerGas), maxPriorityFeePerGas: BigInt(op.maxPriorityFeePerGas) }
			})

	const rundlerCalls = () => bundlerCalls.filter((c) => c.method.startsWith("rundler_")).map((c) => c.method)
	const METHOD_NOT_FOUND = { error: { code: -32601, message: "Method not found" } }

	it("prices a rundler bundler from the fees it suggests, raised by the bumps", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = {
			result: { suggested: { maxPriorityFeePerGas: toHex(2_000_000_000n), maxFeePerGas: toHex(5_000_000_000n) } },
		}

		await expect(sponsorWith("http://rundler.test/bundler")).resolves.not.toBeNull()

		const fees = { maxPriorityFeePerGas: 2_160_000_000n, maxFeePerGas: 5_500_000_000n }
		expect(sentFees()).toEqual([fees])
		expect(vi.mocked(buildPaymasterAndData).mock.calls[0][0].prefund?.maxFeePerGas).toBe(fees.maxFeePerGas)
		expect(rundlerCalls()).toEqual(["rundler_getUserOperationGasPrice"])
		expect(rundlerUserOperationFees).toHaveBeenCalledWith("http://rundler.test/bundler", expect.anything(), expect.anything())
	})

	it("raises the fees to a rundler bundler's priority fee where it suggests none", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = METHOD_NOT_FOUND
		// 30 gwei, the Polygon Amoy floor, against a 1 gwei chain gas price.
		bundlerReplies.rundler_maxPriorityFeePerGas = { result: "0x6fc23ac00" }

		await expect(sponsorWith("http://older-rundler.test/bundler")).resolves.not.toBeNull()

		const fees = { maxPriorityFeePerGas: 32_400_000_000n, maxFeePerGas: 990_000_000n + 32_400_000_000n }
		expect(sentFees()).toEqual([fees])
	})

	it("keeps the chain estimate and stops asking when the bundler does not serve rundler fees", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = METHOD_NOT_FOUND
		bundlerReplies.rundler_maxPriorityFeePerGas = METHOD_NOT_FOUND

		await sponsorWith("http://other-bundler.test/bundler")
		await sponsorWith("http://other-bundler.test/bundler")

		const chainFees = { maxPriorityFeePerGas: 1_080_000_000n, maxFeePerGas: 1_100_000_000n }
		expect(sentFees()).toEqual([chainFees, chainFees])
		expect(rundlerCalls()).toEqual(["rundler_getUserOperationGasPrice", "rundler_maxPriorityFeePerGas"])
	})
})

describe("UserOpSender submitted but unconfirmed", () => {
	const USER_OP_HASH = `0x${"ab".repeat(32)}` as HexString

	const sponsor = async (nonceKey?: bigint) => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		const sender = new UserOpSender(clientManager, configService, signer)
		return sender.trySendSponsored({ chain: CHAIN, callData: "0x" as HexString, nonceKey, gas: GAS })
	}

	const lookup = (service = configService) =>
		new UserOpSender(clientManager, service, signer).lookupUserOpReceipt(CHAIN, USER_OP_HASH)

	it("throws a SubmittedUnconfirmedError carrying the op hash, nonce and nonce key", async () => {
		// A receipt the bundler indexed but that carries no tx hash: the op is unmined.
		bundlerReplies.eth_getUserOperationReceipt = { result: { success: true } }

		const error = await sponsor(7n).catch((err) => err)

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		// The message is the one callers already matched on before the type existed.
		expect(error.message).toBe(`Sponsored UserOp ${USER_OP_HASH} submitted but no receipt within timeout`)
		expect(error.chain).toBe(CHAIN)
		expect(error.submission).toMatchObject({
			kind: "userOp",
			userOpHash: USER_OP_HASH,
			nonce: 0n,
			nonceKey: 7n,
			// Permit2 mode reports a validity deadline, so the op expires.
			expires: true,
		})
		expect(error.submission.submittedAt).toBeTypeOf("number")
	})

	it("marks a bootstrap op as not expiring: its permit deadline is unbounded", async () => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		bundlerReplies.eth_getUserOperationReceipt = { result: { success: true } }
		const sender = new UserOpSender(clientManager, configService, signer)

		const error = await sender
			.trySendSponsored({ chain: CHAIN, callData: "0x" as HexString, gas: GAS, permitBootstrap: true })
			.catch((err) => err)

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.submission).toMatchObject({ kind: "userOp", expires: false })
	})

	it("resolves the op's receipt when the bundler has one", async () => {
		const txHash = `0x${"cd".repeat(32)}` as HexString
		bundlerReplies.eth_getUserOperationReceipt = { result: { success: true, receipt: { transactionHash: txHash } } }

		await expect(lookup()).resolves.toEqual({ txHash, success: true })
	})

	it("reports a reverted op as unsuccessful", async () => {
		const txHash = `0x${"cd".repeat(32)}` as HexString
		bundlerReplies.eth_getUserOperationReceipt = { result: { success: false, receipt: { transactionHash: txHash } } }

		await expect(lookup()).resolves.toEqual({ txHash, success: false })
	})

	it("returns null when the bundler does not know the op", async () => {
		bundlerReplies.eth_getUserOperationReceipt = { result: null }

		await expect(lookup()).resolves.toBeNull()
	})

	it("throws when the bundler cannot be asked", async () => {
		bundlerReplies.eth_getUserOperationReceipt = { error: { message: "boom" } }

		await expect(lookup()).rejects.toThrow("Bundler RPC error (eth_getUserOperationReceipt): boom")
	})

	it("throws when no bundler is configured for the chain", async () => {
		await expect(lookup(configServiceFor(undefined as unknown as string))).rejects.toThrow(
			`No bundler is configured for ${CHAIN}`,
		)
	})
})

/**
 * A lost `eth_sendUserOperation` response must never be read as "never submitted": the op may
 * have been accepted, and a native fallback would execute the same calls twice. These cover the
 * three outcomes of that decision — rejected, unreachable, and ambiguous (recovered or not).
 */
describe("UserOpSender ambiguous send outcome", () => {
	const HTML_PAGE = Symbol("html-page")
	const USER_OP_HASH = `0x${"ab".repeat(32)}` as HexString

	const sponsorWith = (bundlerUrl: string, extra?: Partial<SponsoredUserOpRequest>) => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		const sender = new UserOpSender(clientManager, configServiceFor(bundlerUrl), signer)
		return sender.trySendSponsored({ chain: CHAIN, callData: "0x" as HexString, gas: GAS, ...extra })
	}

	/**
	 * A fetch that answers each method from `byMethod`. A method absent from the map is a
	 * JSON-RPC "method not found"; `{ error }`/`{ result }` are returned with HTTP 200 unless a
	 * numeric `status` rides along, an `Error` value is rejected (a connect-phase failure or an
	 * abort), and `HTML_PAGE` returns a non-JSON 504 body.
	 */
	const stubBundler = (byMethod: Record<string, unknown>) => {
		vi.stubGlobal("fetch", async (_url: unknown, init?: { body?: string; signal?: AbortSignal }) => {
			const { method, params } = JSON.parse(init?.body ?? "{}") as { method: string; params: unknown[] }
			bundlerCalls.push({ method, params, signal: init?.signal })
			const reply = byMethod[method]
			if (reply === undefined) {
				return { status: 404, json: async () => ({ jsonrpc: "2.0", id: 1, error: { code: -32601 } }) }
			}
			if (reply === HTML_PAGE) {
				return {
					status: 504,
					json: async () => {
						throw new SyntaxError("Unexpected token '<'")
					},
				}
			}
			if (reply instanceof Error) throw reply
			const { status, ...body } = reply as { status?: number } & object
			return { status: status ?? 200, json: async () => ({ jsonrpc: "2.0", id: 1, ...body }) }
		})
	}

	/** Skips `findSubmittedOp`'s 2 s / 5 s / 10 s waits without sleeping in real time. */
	const drainLookups = async () => {
		for (const delay of [2_000, 5_000, 10_000]) await vi.advanceTimersByTimeAsync(delay)
	}

	/** Rebuilds the signed packed op from the bundler form of it (an inverse of `prepareBundlerCall`). */
	const packedFromBundler = (op: Record<string, string>) => ({
		sender: op.sender as HexString,
		nonce: BigInt(op.nonce),
		initCode: "0x" as HexString,
		callData: op.callData as HexString,
		accountGasLimits: CryptoUtils.packGasLimits(BigInt(op.verificationGasLimit), BigInt(op.callGasLimit)),
		preVerificationGas: BigInt(op.preVerificationGas),
		gasFees: CryptoUtils.packGasFees(BigInt(op.maxPriorityFeePerGas), BigInt(op.maxFeePerGas)),
		paymasterAndData: `0x${[
			op.paymaster.slice(2),
			BigInt(op.paymasterVerificationGasLimit).toString(16).padStart(32, "0"),
			BigInt(op.paymasterPostOpGasLimit).toString(16).padStart(32, "0"),
			op.paymasterData.slice(2),
		].join("")}` as HexString,
		signature: op.signature as HexString,
	})

	afterEach(() => {
		vi.useRealTimers()
	})

	it("returns null when the bundler answers with a JSON-RPC error", async () => {
		stubBundler({ eth_sendUserOperation: { error: { code: -32500, message: "userop rejected" } } })

		await expect(sponsorWith("http://ambig-reject.test/bundler")).resolves.toBeNull()
		expect(bundlerCalls.some((c) => c.method === "eth_getUserOperationByHash")).toBe(false)
	})

	it("returns null when the connection cannot be established (nothing was sent)", async () => {
		const refused = Object.assign(new TypeError("fetch failed"), { cause: { code: "ECONNREFUSED" } })
		stubBundler({ eth_sendUserOperation: refused })

		await expect(sponsorWith("http://ambig-refused.test/bundler")).resolves.toBeNull()
		expect(bundlerCalls.some((c) => c.method === "eth_getUserOperationByHash")).toBe(false)
	})

	it("a 504 HTML body, then a by-hash hit, then a receipt: resolves with the tx hash", async () => {
		const txHash = `0x${"cd".repeat(32)}` as HexString
		stubBundler({
			eth_sendUserOperation: HTML_PAGE,
			eth_getUserOperationByHash: { result: { userOpHash: USER_OP_HASH } },
			eth_getUserOperationReceipt: { result: { success: true, receipt: { transactionHash: txHash } } },
		})
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-recovered.test/bundler")
		await drainLookups()

		await expect(promise).resolves.toEqual({ txHash })
		expect(bundlerCalls.filter((c) => c.method === "eth_getUserOperationByHash")).toHaveLength(1)
	})

	it("a 504, then by-hash null on every attempt: returns null (never entered the pool)", async () => {
		stubBundler({ eth_sendUserOperation: HTML_PAGE, eth_getUserOperationByHash: { result: null } })
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-absent.test/bundler")
		await drainLookups()

		await expect(promise).resolves.toBeNull()
		expect(bundlerCalls.filter((c) => c.method === "eth_getUserOperationByHash")).toHaveLength(3)
	})

	it("a timeout, then failing lookups too: throws SubmittedUnconfirmedError carrying the local hash", async () => {
		const timeout = Object.assign(new Error("This operation was aborted"), { name: "TimeoutError" })
		const reset = Object.assign(new Error("socket closed"), { cause: { code: "ECONNRESET" } })
		stubBundler({ eth_sendUserOperation: timeout, eth_getUserOperationByHash: reset })
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-unknown.test/bundler").catch((err) => err)
		await drainLookups()
		const error = await promise

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.message).toContain("the bundler's answer was lost and the op could not be looked up")
		expect(error.submission).toMatchObject({ kind: "userOp", nonce: 0n, nonceKey: 0n, expires: true })
		expect(error.submission.submittedAt).toBeTypeOf("number")

		const [lookup] = bundlerCalls.filter((c) => c.method === "eth_getUserOperationByHash")[0].params as [HexString]
		expect(error.submission.userOpHash).toBe(lookup)
	})

	it("the local hash is the EntryPoint hash of the op the bundler received", async () => {
		const timeout = Object.assign(new Error("This operation was aborted"), { name: "TimeoutError" })
		const reset = Object.assign(new Error("socket closed"), { cause: { code: "ECONNRESET" } })
		stubBundler({ eth_sendUserOperation: timeout, eth_getUserOperationByHash: reset })
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-hash.test/bundler").catch((err) => err)
		await drainLookups()
		const error = await promise

		const [sentOp] = bundlerCalls.filter((c) => c.method === "eth_sendUserOperation")[0].params as [
			Record<string, string>,
		]
		const expected = CryptoUtils.computeUserOpHash(packedFromBundler(sentOp), ENTRY_POINT, BigInt(CHAIN_ID))
		expect(error.submission.userOpHash).toBe(expected)
	})

	it("recovers the hash of a 7702-auth op too, which does not change initCode", async () => {
		const reset = Object.assign(new Error("socket closed"), { cause: { code: "ECONNRESET" } })
		stubBundler({ eth_sendUserOperation: reset, eth_getUserOperationByHash: reset })
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-auth.test/bundler", {
			eip7702Auth: async () => ({
				chainId: CHAIN_ID,
				address: DELEGATE,
				nonce: 1,
				r: `0x${"22".repeat(32)}` as HexString,
				s: `0x${"33".repeat(32)}` as HexString,
				yParity: 0,
			}),
		}).catch((err) => err)
		await drainLookups()
		const error = await promise

		const [sentOp] = bundlerCalls.filter((c) => c.method === "eth_sendUserOperation")[0].params as [
			Record<string, string>,
		]
		// The authorization rides beside the op, not in it, so `initCode` stays 0x and the hash is
		// computed from the op alone.
		expect(sentOp.eip7702Auth).toBeDefined()
		expect(error.submission.userOpHash).toBe(
			CryptoUtils.computeUserOpHash(packedFromBundler(sentOp), ENTRY_POINT, BigInt(CHAIN_ID)),
		)
	})

	it("bounds the send with an AbortSignal", async () => {
		stubBundler({ eth_sendUserOperation: { error: { code: -32500, message: "rejected" } } })

		await sponsorWith("http://ambig-signal.test/bundler")

		const send = bundlerCalls.find((c) => c.method === "eth_sendUserOperation")!
		expect(send.signal).toBeInstanceOf(AbortSignal)
	})

	it("treats a proxy's JSON 502 as ambiguous and looks the op up", async () => {
		const txHash = `0x${"cd".repeat(32)}` as HexString
		stubBundler({
			eth_sendUserOperation: { status: 502, error: { code: 504, message: "upstream timed out" } },
			eth_getUserOperationByHash: { result: { userOpHash: USER_OP_HASH } },
			eth_getUserOperationReceipt: { result: { success: true, receipt: { transactionHash: txHash } } },
		})
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-502.test/bundler")
		await drainLookups()

		await expect(promise).resolves.toEqual({ txHash })
		expect(bundlerCalls.filter((c) => c.method === "eth_getUserOperationByHash")).toHaveLength(1)
	})

	it("treats rundler's -32603 internal error as ambiguous and looks the op up", async () => {
		stubBundler({
			eth_sendUserOperation: { error: { code: -32603, message: "internal error" } },
			eth_getUserOperationByHash: { result: null },
		})
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-32603.test/bundler")
		await drainLookups()

		await expect(promise).resolves.toBeNull()
		expect(bundlerCalls.filter((c) => c.method === "eth_getUserOperationByHash")).toHaveLength(3)
	})

	it.each([
		["-32602", -32602],
		["-32500", -32500],
	])("treats a 200 with %s as a definite rejection", async (_label, code) => {
		stubBundler({ eth_sendUserOperation: { error: { code, message: "rejected" } } })

		await expect(sponsorWith(`http://ambig-code-${code}.test/bundler`)).resolves.toBeNull()
		expect(bundlerCalls.some((c) => c.method === "eth_getUserOperationByHash")).toBe(false)
	})

	it("a found op with no receipt by the deadline: throws SubmittedUnconfirmedError from the local hash", async () => {
		const reset = Object.assign(new Error("socket closed"), { cause: { code: "ECONNRESET" } })
		stubBundler({
			eth_sendUserOperation: reset,
			// The bundler answers that it holds the op, but the receipt never appears.
			eth_getUserOperationByHash: { result: { userOpHash: USER_OP_HASH } },
			eth_getUserOperationReceipt: { result: null },
		})
		vi.useFakeTimers()

		const promise = sponsorWith("http://ambig-found-noreceipt.test/bundler").catch((err) => err)
		await drainLookups()
		await vi.advanceTimersByTimeAsync(95_000)
		const error = await promise

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.chain).toBe(CHAIN)
		const [sentOp] = bundlerCalls.filter((c) => c.method === "eth_sendUserOperation")[0].params as [
			Record<string, string>,
		]
		// The local hash, not the different hash the by-hash reply carried.
		expect(error.submission).toMatchObject({
			kind: "userOp",
			nonce: 0n,
			nonceKey: 0n,
			expires: true,
			userOpHash: CryptoUtils.computeUserOpHash(packedFromBundler(sentOp), ENTRY_POINT, BigInt(CHAIN_ID)),
		})
		expect(error.submission.userOpHash).not.toBe(USER_OP_HASH)
	})
})
