import { describe, it, expect, vi, beforeEach } from "vitest"
import { encodeFunctionData, erc20Abi, maxUint256, zeroAddress } from "viem"
import { encodeERC7821ExecuteBatch, type HexString } from "@hyperbridge/sdk"

import { DelegationService } from "@/services/DelegationService"
import { SubmittedUnconfirmedError } from "@/services/submission"
import { resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Which delegation path `setupDelegation` tries first, what the sponsored op carries, and
 * how a delegation is confirmed once a tx has mined.
 *
 * A pending Permit2 approval on a token with no EIP-2612 permit can only ride a native
 * set-code tx, so that goes first when the EOA covers it. A permit-capable token instead
 * bootstraps through the bundler — the op pays for itself with a permit and installs the
 * approval in its own callData — which is preferred even when native is available,
 * because it spends stablecoins rather than native. With nothing pending, the bundler
 * goes first and the op is a plain no-op.
 *
 * The chain mock is stateful: `getCode` reports the delegate the wallet carries now, and
 * a send (or an explicitly landed sponsored op) flips it.
 */

const { trySendSponsored } = vi.hoisted(() => ({ trySendSponsored: vi.fn() }))

vi.mock("@/services/paymaster", () => ({ hasPaymaster: () => true }))
vi.mock("@/services/paymaster/provider/simplex", () => ({ resolvePendingPermit2Approval: vi.fn() }))
vi.mock("@/services/UserOpSender", () => ({
	UserOpSender: class {
		canSponsor() {
			return true
		}
		trySendSponsored = trySendSponsored
	},
}))

const CHAIN = "EVM-56"
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const SOLVER_ACCOUNT = "0x00000000000000000000000000000000000000cc" as HexString
const OLD_SOLVER_ACCOUNT = "0x00000000000000000000000000000000000000bb" as HexString
const PAYMASTER = "0x00000000000000000000000000000000000000aa" as HexString
const USDT = "0x55d398326f99059fF775485246999027B3197955" as HexString
const USDC = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831" as HexString
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as HexString
const GAS_PRICE = 1_000_000_000n
const DIRECT_TX_COST = 650_000n * GAS_PRICE
const DIRECT_TX_HASH = ("0x" + "ab".repeat(32)) as HexString
const SPONSORED_TX_HASH = ("0x" + "cd".repeat(32)) as HexString

const configService = {
	getSolverAccountContractAddress: () => SOLVER_ACCOUNT,
	getChainId: () => 56,
	getSimplexPaymasterAddress: () => PAYMASTER,
	loggers: undefined,
} as unknown as FillerConfigService

const signer = {
	address: SOLVER,
	mode: "test",
	signAuthorization: async () => ({ r: "0x01" as HexString, s: "0x02" as HexString, yParity: 0 }),
} as unknown as Signer

function build(opts: {
	native: bigint
	receiptStatus?: "success" | "reverted"
	/** The delegate the wallet carries before any tx lands. */
	delegatedTo?: HexString
	/**
	 * The delegate a landed tx leaves behind. Defaults to the SolverAccount, so a
	 * delegation that succeeds reads back; `null` leaves no code at all.
	 */
	delegateAfterSend?: HexString | null
	/** Nonce reported at `latest`. */
	latestNonce?: number
	/** Nonce reported at `pending`; defaults to `latest`. */
	pendingNonce?: number
	/** Make `getCode` throw on a read pinned to a block, to exercise a failed confirmation. */
	codeReadError?: boolean
	/** Make the bundler path's fresh-EOA `getCode` read (the second unblocked read) throw. */
	freshReadError?: boolean
}) {
	let unblockedReads = 0
	const state = {
		delegatedTo: opts.delegatedTo,
		/** The authority's nonce as finalised. */
		nonce: opts.latestNonce ?? 0,
		/** The authority's nonce including the mempool. */
		pendingNonce: opts.pendingNonce ?? opts.latestNonce ?? 0,
	}
	/** The code a landed delegation leaves. `null` means no code at all. */
	const landedCode = () =>
		opts.delegateAfterSend === undefined ? SOLVER_ACCOUNT : (opts.delegateAfterSend ?? undefined)

	const sendTransaction = vi.fn(async (tx: { nonce?: number; authorizationList?: { nonce: number }[] }) => {
		// EIP-7702 applies the tuple only when it is the one after the tx's nonce; otherwise
		// the tx still mines and reads success while the code is unchanged. A reverted tx
		// lands nothing either way.
		const tuple = tx.authorizationList?.[0]
		const applied = tuple !== undefined && tx.nonce === state.pendingNonce && tuple.nonce === tx.nonce + 1
		if (applied && opts.receiptStatus !== "reverted") state.delegatedTo = landedCode()
		return DIRECT_TX_HASH
	})
	const publicClient = {
		// A delegated EOA carries the EIP-7702 indicator followed by the delegate address.
		getCode: async (args: { blockNumber?: bigint } = {}) => {
			if (opts.codeReadError && args.blockNumber !== undefined) throw new Error("pinned-block read failed")
			if (args.blockNumber === undefined) {
				unblockedReads += 1
				// The first unblocked read is isDelegated; the second is the bundler path's.
				if (opts.freshReadError && unblockedReads > 1) throw new Error("fresh-EOA read failed")
			}
			return state.delegatedTo ? `0xef0100${state.delegatedTo.slice(2)}` : "0x"
		},
		getBalance: async () => opts.native,
		getGasPrice: async () => GAS_PRICE,
		getTransactionCount: async (args: { blockTag?: string } = {}) =>
			(args.blockTag ?? "latest") === "pending" ? state.pendingNonce : state.nonce,
		waitForTransactionReceipt: async () => ({ status: opts.receiptStatus ?? "success", blockNumber: 1n }),
		getTransactionReceipt: async () => ({ status: opts.receiptStatus ?? "success", blockNumber: 1n }),
	}
	const clientManager = {
		loggers: undefined,
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ chain: undefined, sendTransaction }),
	} as unknown as ChainClientManager
	return {
		service: new DelegationService(clientManager, configService, signer),
		sendTransaction,
		/** Move the wallet's on-chain code, for tests where the sponsored op is mocked. */
		setDelegated: (to?: HexString) => {
			state.delegatedTo = to
		},
		/**
		 * Apply the delegation a landed sponsored op leaves — but only when its tuple is the
		 * account's current nonce, as EIP-7702 requires.
		 */
		landSponsoredOp: (tuple?: { nonce: number }) => {
			if (tuple && tuple.nonce === state.nonce) state.delegatedTo = landedCode()
		},
	}
}

const batchedApprove = encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [PERMIT2, maxUint256] })

/** The exact ERC-7821 payload the bootstrap op must carry — every byte, not just the addresses. */
const bootstrapCallData = encodeERC7821ExecuteBatch([
	{
		target: USDC,
		value: 0n,
		data: encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [PERMIT2, maxUint256] }),
	},
])

/**
 * A sponsored op the bundler accepted. The delegation lands only if the signed tuple is the
 * account's current nonce — the same rule the chain applies — so a regression in the tuple
 * nonce fails the confirmation, not just the nonce assertion.
 */
function sponsoredOpResolves(land: (tuple?: { nonce: number }) => void) {
	trySendSponsored.mockImplementation(async (req: { eip7702Auth?: () => Promise<{ nonce: number }> }) => {
		land(await req.eip7702Auth?.())
		return { txHash: SPONSORED_TX_HASH }
	})
}

function unconfirmed(): SubmittedUnconfirmedError {
	return new SubmittedUnconfirmedError("Sponsored UserOp submitted but no receipt within timeout", CHAIN, {
		kind: "userOp",
		userOpHash: SPONSORED_TX_HASH,
		nonce: 0n,
		nonceKey: 0n,
		expires: false,
		submittedAt: Date.now(),
	})
}

beforeEach(() => {
	trySendSponsored.mockReset()
	vi.mocked(resolvePendingPermit2Approval).mockReset()
})

describe("setupDelegation ordering", () => {
	it("sends the batched delegate+approve first when a Permit2 approval is pending and native covers it", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [{ to: string; data: string }]
		expect(tx.to).toBe(USDT)
		expect(tx.data).toBe(batchedApprove)
	})

	it("still tries the bundler first when the approval is pending but native is short", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction, landSponsoredOp } = build({ native: DIRECT_TX_COST - 1n })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).toHaveBeenCalledOnce()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("tries the bundler first when nothing is pending", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction, landSponsoredOp } = build({ native: DIRECT_TX_COST })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).toHaveBeenCalledOnce()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("bootstraps a permit-capable token through the bundler even when native covers a direct tx", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		const { service, sendTransaction, landSponsoredOp } = build({ native: DIRECT_TX_COST })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// No native spent: the permit pays for the op that installs the allowance.
		expect(sendTransaction).not.toHaveBeenCalled()
		expect(trySendSponsored).toHaveBeenCalledOnce()

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.permitBootstrap).toBe(true)
		// Exact bytes, not a substring match: both addresses appear in the encoding however the
		// call is mis-assembled, so `toContain` would pass on a wrong approve amount, a wrong
		// target, or a missing ERC-7821 wrapper. A 1-wei allowance in particular would strand
		// the chain forever — resolvePendingPermit2Approval refuses any non-zero allowance.
		expect(req.callData).toBe(bootstrapCallData)
	})

	it("sends a plain no-op op, with no permit bootstrap, once the allowance is in place", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, landSponsoredOp } = build({ native: DIRECT_TX_COST })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.callData).toBe("0x")
		expect(req.permitBootstrap).toBe(false)
	})

	it("does not ask for a permit bootstrap on a no-permit token", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, landSponsoredOp } = build({ native: DIRECT_TX_COST - 1n })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.callData).toBe("0x")
		expect(req.permitBootstrap).toBe(false)
	})

	it("installs the Permit2 allowance on an already-delegated account instead of returning early", async () => {
		// The regression this guards: an account delegated by a release that charged EIP-2612
		// permits has NO Permit2 allowance, and setupDelegation used to return the moment
		// isDelegated held — stranding every upgraded solver on a native-funded approve.
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		trySendSponsored.mockResolvedValue({ txHash: SPONSORED_TX_HASH })
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).not.toHaveBeenCalled()
		expect(trySendSponsored).toHaveBeenCalledOnce()
		const [req] = trySendSponsored.mock.calls[0] as unknown as [
			{ callData: HexString; permitBootstrap: boolean; eip7702Auth?: unknown },
		]
		expect(req.permitBootstrap).toBe(true)
		expect(req.callData).toBe(bootstrapCallData)
		// Already delegated, so the op carries no authorization — only the approve.
		expect(req.eip7702Auth).toBeUndefined()
	})

	it("sends nothing on an already-delegated account whose allowance is already in place", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("leaves a no-permit token on an already-delegated account to the funded approve", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// Nothing to pay the op with, so no op — the first sponsored op sends the approve.
		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("still reports the account delegated when the allowance op fails", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		trySendSponsored.mockRejectedValue(new Error("bundler down"))
		const { service } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		// Best-effort: a failed bootstrap must not turn a delegated account into a setup failure.
		expect(await service.setupDelegation(CHAIN)).toBe(true)
	})

	it("falls through to the bundler when the batched tx reverts without delegating", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction, landSponsoredOp } = build({
			native: DIRECT_TX_COST,
			receiptStatus: "reverted",
		})
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// The batched attempt went out first, then the sponsored path took over.
		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(trySendSponsored).toHaveBeenCalledOnce()
	})
})

describe("delegation nonce", () => {
	it("sends the plain direct tx at the pending nonce and signs the tuple one later", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, latestNonce: 4, pendingNonce: 5 })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		const [tx] = sendTransaction.mock.calls[0] as unknown as [
			{ nonce: number; authorizationList: { nonce: number }[] },
		]
		expect(tx.nonce).toBe(5)
		expect(tx.authorizationList[0].nonce).toBe(6)
	})

	it("sends the batched delegate+approve tx at the pending nonce and signs the tuple one later", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, latestNonce: 4, pendingNonce: 5 })

		expect(await service.setupDelegation(CHAIN)).toBe(true)
		expect(trySendSponsored).not.toHaveBeenCalled()

		const [tx] = sendTransaction.mock.calls[0] as unknown as [
			{ nonce: number; authorizationList: { nonce: number }[] },
		]
		expect(tx.nonce).toBe(5)
		expect(tx.authorizationList[0].nonce).toBe(6)
	})

	it("revokes at the pending nonce and signs the tuple one later", async () => {
		const { service, sendTransaction } = build({
			native: 0n,
			latestNonce: 4,
			pendingNonce: 5,
			delegatedTo: SOLVER_ACCOUNT,
			delegateAfterSend: null,
		})

		expect(await service.revokeDelegation(CHAIN)).toBe(true)

		const [tx] = sendTransaction.mock.calls[0] as unknown as [
			{ nonce: number; authorizationList: { nonce: number }[] },
		]
		expect(tx.nonce).toBe(5)
		expect(tx.authorizationList[0].nonce).toBe(6)
	})

	it("refuses to sign a bundler authorization while a tx is in flight, and falls back to native", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		// Mirrors UserOpSender: the authorization factory runs inside its preparation
		// try/catch, so a throw there is "never submitted" and the caller may fall back.
		let factoryError: unknown
		trySendSponsored.mockImplementation(async (req: { eip7702Auth?: () => Promise<unknown> }) => {
			try {
				await req.eip7702Auth?.()
			} catch (error) {
				factoryError = error
				return null
			}
			return { txHash: SPONSORED_TX_HASH }
		})
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, latestNonce: 4, pendingNonce: 5 })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(String(factoryError)).toMatch(/transaction in flight/)
		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [
			{ nonce: number; authorizationList: { nonce: number }[] },
		]
		expect(tx.nonce).toBe(5)
		expect(tx.authorizationList[0].nonce).toBe(6)
	})

	it("fails without sending when a bundler authorization is refused and there is no native", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockImplementation(async (req: { eip7702Auth?: () => Promise<unknown> }) => {
			try {
				await req.eip7702Auth?.()
			} catch {
				return null
			}
			return { txHash: SPONSORED_TX_HASH }
		})
		const { service, sendTransaction } = build({ native: 0n, latestNonce: 4, pendingNonce: 5 })

		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).not.toHaveBeenCalled()
	})
})

describe("confirming a delegation on chain", () => {
	it("does not report success when a successful direct tx left no delegation", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, delegateAfterSend: null })

		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(trySendSponsored).toHaveBeenCalledOnce()
	})

	it("does not confirm a delegation when the pinned-block read fails", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, codeReadError: true })

		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).toHaveBeenCalledOnce()
	})

	it("lets a pre-submit read failure reach the caller instead of reporting failed", async () => {
		// The fresh-EOA read happens before the op is built. Nothing was submitted, so a throw
		// there must reach the caller to retry, not be swallowed into "failed" (no fallback).
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue({ txHash: SPONSORED_TX_HASH })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, freshReadError: true })

		await expect(service.setupDelegation(CHAIN)).rejects.toThrow(/fresh-EOA read failed/)
		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})
})

describe("setupDelegation after a SolverAccount upgrade", () => {
	it("reports a sponsored re-delegation that mined but still shows the old delegate as failed", async () => {
		// The realistic failure: a tuple signed for a stale nonce is skipped, the op validates
		// against the OLD delegate and the bundler reports success — while the wallet is still
		// not delegated to the new SolverAccount. Nothing may fall back to a native tx.
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction, landSponsoredOp } = build({
			native: 0n,
			delegatedTo: OLD_SOLVER_ACCOUNT,
			delegateAfterSend: OLD_SOLVER_ACCOUNT,
		})
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.isDelegated(CHAIN)).toBe(false)
		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).not.toHaveBeenCalled()
		expect(trySendSponsored).toHaveBeenCalledOnce()
		const [req] = trySendSponsored.mock.calls[0] as unknown as [
			{ eip7702Auth?: () => Promise<{ address: HexString }> },
		]
		expect(req.eip7702Auth).toBeTypeOf("function")
		expect((await req.eip7702Auth!()).address).toBe(SOLVER_ACCOUNT)
	})

	it("reports a sponsored re-delegation that landed the new SolverAccount", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction, landSponsoredOp } = build({ native: 0n, delegatedTo: OLD_SOLVER_ACCOUNT })
		sponsoredOpResolves(landSponsoredOp)

		expect(await service.isDelegated(CHAIN)).toBe(false)
		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("re-delegates through a direct tx when the sponsored path is unavailable", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, delegatedTo: OLD_SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [{ authorizationList: { address: HexString }[] }]
		expect(tx.authorizationList.map((auth) => auth.address)).toEqual([SOLVER_ACCOUNT])
	})

	it("leaves a wallet already delegated to the new SolverAccount alone", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.isDelegated(CHAIN)).toBe(true)
		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})
})

describe("no native fallback after a sponsored op that may still land or did land", () => {
	it("stops at pending when a submitted op is unconfirmed and nothing is delegated yet", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockRejectedValue(unconfirmed())
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("reports delegated when an unconfirmed op turns out to have landed", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction, setDelegated } = build({ native: DIRECT_TX_COST })
		trySendSponsored.mockImplementation(async () => {
			// The op landed while the caller waited for a receipt it never got.
			setDelegated(SOLVER_ACCOUNT)
			throw unconfirmed()
		})

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("stops after a sponsored op reverted on chain, without a native retry", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockRejectedValue(
			new Error(`Sponsored UserOp ${SPONSORED_TX_HASH} reverted on-chain (tx ${DIRECT_TX_HASH})`),
		)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(false)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("still falls back to native when the sponsored op was never submitted", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).toHaveBeenCalledOnce()
	})
})

describe("one delegation attempt per wallet and chain", () => {
	it("shares one in-flight attempt between two services, and releases it once it settles", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue(null)
		// Two services over the same wallet and chain, as the filler and the limit-order
		// service are. Neither sees the other's attempt through its own state.
		const a = build({ native: DIRECT_TX_COST, delegateAfterSend: null })
		const b = build({ native: DIRECT_TX_COST, delegateAfterSend: null })

		const [ra, rb] = await Promise.all([a.service.setupDelegation(CHAIN), b.service.setupDelegation(CHAIN)])

		expect(ra).toBe(false)
		expect(rb).toBe(false)
		expect(trySendSponsored).toHaveBeenCalledOnce()
		expect(a.sendTransaction).toHaveBeenCalledOnce()
		expect(b.sendTransaction).not.toHaveBeenCalled()

		// The entry is gone once the attempt settles, so a later call runs for real again.
		expect(await a.service.setupDelegation(CHAIN)).toBe(false)
		expect(trySendSponsored).toHaveBeenCalledTimes(2)
	})

	it("queues a revoke behind an in-flight setup instead of overlapping it", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		let release!: () => void
		const gate = new Promise<void>((resolve) => {
			release = resolve
		})
		const { service, sendTransaction, landSponsoredOp } = build({ native: 0n })
		trySendSponsored.mockImplementation(async (req: { eip7702Auth?: () => Promise<{ nonce: number }> }) => {
			await gate
			landSponsoredOp(await req.eip7702Auth?.())
			return { txHash: SPONSORED_TX_HASH }
		})

		const setup = service.setupDelegation(CHAIN)
		const revoke = service.revokeDelegation(CHAIN)
		// The revoke shares the key, so it cannot run while the setup holds the lock.
		expect(sendTransaction).not.toHaveBeenCalled()

		release()
		expect(await setup).toBe(true)
		// Then it runs, and it is the revoke's own tx — the zero address, not the setup's.
		expect(await revoke).toBe(false)
		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [{ authorizationList: { address: HexString }[] }]
		expect(tx.authorizationList[0].address).toBe(zeroAddress)
	})
})
