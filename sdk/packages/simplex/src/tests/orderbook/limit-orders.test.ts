import { describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { OrderbookRequestError } from "@/orderbook/client"
import { initialOrderNonce, LimitOrderValidationError, type CreateLimitOrderRequest } from "@/orderbook/limit-orders"
import type { CancelOrderResult } from "@/orderbook/types"
import {
	CREATE_REQUEST as REQUEST,
	LIMITS,
	fakeClient,
	limitOrderService as makeService,
	ORDERBOOK_FIXTURES,
	postedOrder,
} from "../helpers/limit-orders"

const { ONE, CHAIN } = ORDERBOOK_FIXTURES

describe("LimitOrderService.create", () => {
	it("stores the order and records the orderbook's posting", async () => {
		const { service, store } = makeService(fakeClient([]))
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("accepted")
		expect(order.status).toBe("open")
		expect(order.commitment).toBe("0xabc")
		// The rate follows from the two amounts: 1,500,000 cNGN for 1,000 USDC.
		// The orderbook shades a posting by the protocol fee, so both are kept.
		expect(order.side).toBe("BID")
		expect(order.price).toBe((1500n * ONE).toString())
		expect(order.bookPrice).toBe((1490n * ONE).toString())
		expect(order.remaining).toBe(order.size)
		expect(await store.get(order.id)).toEqual(order)
	})

	it("finds the book however the symbols are cased, and keeps the book's spelling", async () => {
		// The asset registry upper-cases symbols (CNGN) where the orderbook lists cNGN, so the
		// operator UI asked for a book the lookup could not see.
		const { service } = makeService(fakeClient([]))
		const { order, result } = await service.create({ ...REQUEST, tokenIn: "usdc", tokenOut: "cngn" })

		expect(result.kind).toBe("accepted")
		expect(order.book).toBe("USDC/CNGN")
		expect([order.base, order.quote, order.side]).toEqual(["USDC", "CNGN", "BID"])
		expect(order.price).toBe((1500n * ONE).toString())
	})

	it("keeps a rejected order with the reason on it rather than dropping the request", async () => {
		const client = fakeClient([{ kind: "rejected", code: "UNSUPPORTED_PAIR", message: "no such market" }])
		const { service, store } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("rejected")
		expect(order.status).toBe("rejected")
		expect(order.lastError).toBe("UNSUPPORTED_PAIR: no such market")
		expect(await store.get(order.id)).not.toBeNull()
	})

	it("bumps the nonce and reposts once when the orderbook has seen the op before", async () => {
		const client = fakeClient([{ kind: "rejected", code: "REPLAYED", message: "seen" }])
		const { service } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("accepted")
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(order.orderNonce).toBe("1")
		expect(order.status).toBe("open")
	})

	it("starts orders on the same terms at different nonces, so their posted ops differ", async () => {
		// The op is built from the terms and the nonce alone. Every order used to start at 0, and a
		// third order on the terms of two earlier ones found 0 and 1 both REPLAYED and was refused.
		const client = fakeClient([])
		const { service } = makeService(client, undefined, {}, initialOrderNonce)
		const first = await service.create(REQUEST)
		const second = await service.create(REQUEST)

		expect(first.order.orderNonce).not.toBe(second.order.orderNonce)
		expect(new Set(client.submitted).size).toBe(2)
	})

	it("does not retry a rejection a new nonce cannot fix", async () => {
		const client = fakeClient([{ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }])
		const { service } = makeService(client)
		await service.create(REQUEST)
		expect(client.submitted).toHaveLength(1)
	})

	it("leaves an order the orderbook could not decide open, to be posted again", async () => {
		// A refusal retires the order; a failure must not. One timeout on the way in
		// would otherwise kill an order the operator still wants, with nothing left
		// looking at the row.
		const client = fakeClient([])
		client.submitOrder = async () => {
			throw new OrderbookRequestError("connect ECONNREFUSED")
		}
		const { service, store } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result).toMatchObject({ kind: "failed", retryable: true })
		expect(order.status).toBe("open")
		expect(order.commitment).toBeNull()
		expect(order.lastError).toMatch(/REQUEST_FAILED/)
		expect((await store.get(order.id))?.status).toBe("open")
	})

	it("retires an order the orderbook actually refused", async () => {
		const client = fakeClient([{ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }])
		const { service } = makeService(client)
		const { order } = await service.create(REQUEST)
		expect(order.status).toBe("rejected")
	})
})

describe("an op the orderbook has already taken", () => {
	it("treats a live entry at the same commitment as the posting it was", async () => {
		// `ORDER_EXISTS` is a live entry sitting at that commitment, unlike
		// `REPLAYED`. Posting again on a new nonce would leave two entries behind one
		// liability and record only the second.
		const client = fakeClient([{ kind: "rejected", code: "ORDER_EXISTS", message: "already here" }])
		client.entries = [postedOrder({ commitment: "0xabc" })]
		const { service } = makeService(client)

		const { order, result } = await service.create(REQUEST)
		expect(result.kind).toBe("unchanged")
		expect(order.status).toBe("open")
		expect(order.commitment).toBe("0xabc")
		// One submission, not two.
		expect(client.submitted).toEqual(["0x00"])
	})

	it("bumps the nonce when the orderbook holds no such entry", async () => {
		const client = fakeClient([{ kind: "rejected", code: "REPLAYED", message: "seen" }])
		const { service } = makeService(client)

		const { order } = await service.create(REQUEST)
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(order.orderNonce).toBe("1")
	})
})

describe("LimitOrderService.create validation", () => {
	const rejects = async (patch: Partial<CreateLimitOrderRequest>, match: RegExp) => {
		const { service } = makeService(fakeClient([]))
		await expect(service.create({ ...REQUEST, ...patch })).rejects.toThrow(match)
	}

	it("refuses a missing or empty accepted-sources list, which the orderbook would reject", async () => {
		await rejects({ acceptedSources: [] }, /at least one source chain/)
		await rejects({ acceptedSources: undefined as unknown as string[] }, /at least one source chain/)
	})

	it("refuses a repeated source chain", async () => {
		await rejects({ acceptedSources: ["EVM-1", "EVM-1"] }, /must not repeat/)
	})

	it("refuses an amountOut under the paid token's dust floor", async () => {
		await rejects({ amountOut: "999" }, /dust floor for CNGN/)
	})

	it("refuses a ttl under the orderbook's minimum", async () => {
		await rejects({ ttlSecs: 60 }, /at least the orderbook's minimum of 900/)
	})

	it("refuses a pair no book trades, naming the ones on offer", async () => {
		await rejects({ tokenOut: "EURC" }, /No book trades USDC against EURC.*USDC\/CNGN/)
	})

	it("refuses a same-asset order that pays out more than it takes in", async () => {
		await rejects(
			{ tokenOut: "USDC", amountOut: "1001", amountIn: "1000" },
			/must pay out no more than it takes in/,
		)
	})

	it("refuses a chain this filler does not run", async () => {
		await rejects({ fillChain: "EVM-1" }, /not a chain this filler is configured for/)
	})

	it("refuses a source chain the orderbook does not serve", async () => {
		// Half of what UNSUPPORTED_SOURCE_CHAIN tests, and the half an operator gets
		// wrong by typo, so it is worth catching before a row is stored.
		await rejects({ acceptedSources: ["EVM-1", "EVM-42161"] }, /does not serve EVM-42161/)
	})

	it("refuses an amount that is not a positive decimal in whole tokens", async () => {
		await rejects({ amountIn: "0" }, /amountIn must be greater than zero/)
		await rejects({ amountOut: "1.5e3" }, /amountOut must be an amount in whole tokens/)
		await rejects({ amountOut: "0.0000000000000000001" }, /more than 18 decimal places/)
	})
})

describe("what the operator states", () => {
	it("is whole tokens, scaled to the orderbook's own unit on the way in", async () => {
		// Nobody creating an order should have to know the asset's decimals, let
		// alone that the orderbook normalises everything to 1e18.
		const { service, store } = makeService(fakeClient([{ kind: "accepted", order: postedOrder(), surfaced: true }]))

		const { order } = await service.create({ ...REQUEST, amountIn: "1000.5", amountOut: "1500750" })

		const stored = (await store.get(order.id))!
		expect(stored.size).toBe((1_500_750n * ONE).toString())
		// 1,500,750 cNGN for 1,000.5 USDC is 1,500 cNGN per USDC.
		expect(stored.price).toBe((1500n * ONE).toString())
	})
})

describe("what the wallet can actually pay", () => {
	it("refuses an order the balance cannot cover", async () => {
		// The orderbook backs an entry with the solver's real balance and cuts down
		// what it is not holding, so an order written against money that is not there
		// is refused or silently shrunk rather than filled.
		const { service } = makeService(fakeClient([]), undefined, { [ORDERBOOK_FIXTURES.CNGN]: 1_000_000n })

		await expect(service.create({ ...REQUEST, amountOut: "1500000" })).rejects.toThrow(
			/holds 1000000 CNGN on EVM-8453, which cannot pay out 1500000/,
		)
	})

	/**
	 * A fill pays out of the wallet and withdraws any shortfall from the configured vaults in the
	 * same batch, so vault holdings back an order as much as the wallet does. Counting the wallet
	 * alone refused orders the filler would have filled, which is every order once an operator
	 * sweeps inventory into a vault.
	 */
	describe("with inventory in a vault", () => {
		/** A vault position of `positionAssets` cNGN on the fill chain, in whole tokens. */
		const vaultHolding = (whole: bigint, overrides: Record<string, unknown> = {}) => ({
			getBalanceSnapshot: async () => [
				{
					chain: CHAIN,
					vault: "0x9999999999999999999999999999999999999999" as HexString,
					asset: ORDERBOOK_FIXTURES.CNGN,
					symbol: "CNGN",
					decimals: 18,
					positionAssets: whole * 10n ** 18n,
					availableAssets: whole * 10n ** 18n,
					walletReserve: 0n,
					acceptsDeposits: true,
					...overrides,
				},
			],
		})

		it("backs an order the wallet alone could not pay", async () => {
			const { service } = makeService(
				fakeClient([{ kind: "accepted", order: postedOrder(), surfaced: true }]),
				undefined,
				{ [ORDERBOOK_FIXTURES.CNGN]: 1_000_000n },
				undefined,
				vaultHolding(600_000n),
			)

			const { order } = await service.create({ ...REQUEST, amountOut: "1500000" })

			expect(order.size).toBe((1_500_000n * ONE).toString())
		})

		it("still refuses what the wallet and the vaults together cannot pay, and says what is where", async () => {
			const { service } = makeService(fakeClient([]), undefined, { [ORDERBOOK_FIXTURES.CNGN]: 1_000_000n }, undefined, vaultHolding(200_000n))

			await expect(service.create({ ...REQUEST, amountOut: "1500000" })).rejects.toThrow(
				/holds 1000000 in the wallet and 200000 in vaults CNGN on EVM-8453, which cannot pay out 1500000/,
			)
		})

		it("counts only vaults holding the payout token on the fill chain", async () => {
			const otherToken = makeService(fakeClient([]), undefined, { [ORDERBOOK_FIXTURES.CNGN]: 1_000_000n }, undefined, {
				getBalanceSnapshot: async () => [
					{
						chain: CHAIN,
						vault: "0x9999999999999999999999999999999999999999" as HexString,
						asset: ORDERBOOK_FIXTURES.USDC,
						symbol: "USDC",
						decimals: 6,
						positionAssets: 600_000n * 10n ** 6n,
						availableAssets: 600_000n * 10n ** 6n,
						walletReserve: 0n,
						acceptsDeposits: true,
					},
				],
			})

			await expect(otherToken.service.create({ ...REQUEST, amountOut: "1500000" })).rejects.toThrow(
				/cannot pay out 1500000/,
			)
		})

		it("falls back to the wallet when the vault snapshot cannot be read", async () => {
			const { service } = makeService(fakeClient([]), undefined, { [ORDERBOOK_FIXTURES.CNGN]: 1_000_000n }, undefined, {
				getBalanceSnapshot: async () => {
					throw new Error("rpc down")
				},
			})

			await expect(service.create({ ...REQUEST, amountOut: "1500000" })).rejects.toThrow(
				/holds 1000000 CNGN on EVM-8453, which cannot pay out 1500000/,
			)
		})
	})

	it("lets several orders rest on the same balance", async () => {
		// One balance backs every order resting on it, which is what quoting both
		// sides of a book is. The orderbook says the same: it advertises each entry
		// at `min(quoted, balance)` rather than dividing the balance between them,
		// and whichever fills first draws the inventory down.
		const client = fakeClient([
			{ kind: "accepted", order: postedOrder({ commitment: "0xa1" }), surfaced: true },
			{ kind: "accepted", order: postedOrder({ commitment: "0xa2" }), surfaced: true },
		])
		const { service } = makeService(client, undefined, { [ORDERBOOK_FIXTURES.CNGN]: 2_000_000n })

		await service.create({ ...REQUEST, amountOut: "1500000" })

		await expect(service.create({ ...REQUEST, amountOut: "1500000" })).resolves.toBeDefined()
	})
})

describe("token decimals", () => {
	it("takes them from the orderbook's own registry rather than the chain", async () => {
		// The server prices against this registry, and every post and repost needs
		// both sides, so the limits already cached here save two chain reads each
		// time and agree with what the orderbook expects by construction.
		const client = fakeClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)

		// 1,500,000 cNGN at the fixture's 18 decimals, where the chain stub would
		// have said 18 for cNGN too but 6 for USDC on the input side.
		expect(client.submitted).toEqual(["0x00"])
	})

	it("falls back to the token when the orderbook lists neither the chain nor the symbol", async () => {
		const client = fakeClient([])
		client.limits = async () => ({ ...LIMITS, chains: [] })
		const { service, store } = makeService(client)
		const { order } = await service.create(REQUEST)

		expect(order.status).toBe("open")
		expect((await store.get(order.id))?.commitment).toBe("0xabc")
	})
})

describe("a same-asset limit order", () => {
	/** Take in 1,000 USDC and pay out 999: the ask-only, below-par case curves used to hold. */
	const SAME: CreateLimitOrderRequest = {
		...REQUEST,
		tokenOut: "USDC",
		amountIn: "1000",
		amountOut: "999",
	}

	it("is stored and priced here, and never sent to a book that does not exist", async () => {
		const client = fakeClient([])
		const { service, store } = makeService(client)
		const { order, result } = await service.create(SAME)

		expect(result.kind).toBe("unposted")
		expect(order.status).toBe("open")
		expect(order.base).toBe("USDC")
		expect(order.quote).toBe("USDC")
		expect(order.commitment).toBeNull()
		expect(order.price).toBe(((999n * ONE) / 1000n).toString())
		expect(client.submitted).toEqual([])
		// Stated in whole tokens, stored at the orderbook's 1e18.
		expect((await store.get(order.id))?.remaining).toBe((999n * ONE).toString())
	})

	it("is worked down by a fill without anything being reposted", async () => {
		const client = fakeClient([])
		const { service } = makeService(client)
		const created = await service.create(SAME)

		const settled = await service.settleFill(created.order.id, 500n * ONE)
		expect(settled?.status).toBe("open")
		expect(settled?.remaining).toBe((499n * ONE).toString())
		expect(client.submitted).toEqual([])
	})

	it("is left alone by reconciliation, which has no entry to compare it with", async () => {
		const client = fakeClient([])
		const { service } = makeService(client)
		await service.create(SAME)

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.submitted).toEqual([])
	})
})

describe("LimitOrderService.settleFill", () => {
	/** Take in 1,000 USDC, pay out 1,500,000 cNGN, then see 500,000 cNGN go out. */
	const fill = async (delivered: bigint, cancels: CancelOrderResult[] = []) => {
		const client = fakeClient([], cancels)
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		const settled = await service.settleFill(created.order.id, delivered)
		return { client, store, settled: settled!, id: created.order.id }
	}

	it("reports a resize and a close, which nobody asked for", async () => {
		// Every other change to a limit order is something the operator initiated and
		// gets told about by the controller that took the request. These two happen
		// inside a fill, so without this the size just goes stale on their screen.
		const client = fakeClient([])
		const { service } = makeService(client)
		const events: string[] = []
		service.listen((event) => events.push(`${event.kind}:${event.order.remaining}`))
		const created = await service.create(REQUEST)

		await service.settleFill(created.order.id, 500_000n * ONE)
		expect(events).toEqual([`resized:${(1_000_000n * ONE).toString()}`])

		await service.settleFill(created.order.id, 999_999n * ONE)
		expect(events[1]).toMatch(/^filled:/)
	})

	it("works the order down by what went out and reposts the rest", async () => {
		const { client, settled } = await fill(500_000n * ONE)

		expect(settled.remaining).toBe((1_000_000n * ONE).toString())
		expect(settled.status).toBe("open")
		// A fresh op on a fresh nonce: the orderbook remembers every hash it took.
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(settled.orderNonce).toBe("1")
	})

	it("closes the order when what is left falls under the dust floor", async () => {
		// 1,499,500 of 1,500,000 delivered leaves 500, under the 1,000 floor.
		const { client, settled } = await fill(1_499_500n * ONE)

		expect(settled.status).toBe("filled")
		expect(settled.commitment).toBeNull()
		// Cancelled, not reposted.
		expect(client.submitted).toEqual(["0x00"])
	})

	it("cancels the old entry before posting the new one", async () => {
		// Two live entries for one liability would advertise the same output twice.
		const order: string[] = []
		const client = fakeClient([])
		const submit = client.submitOrder
		client.submitOrder = async (userOp) => {
			order.push("submit")
			return submit(userOp)
		}
		client.cancelOrder = async () => {
			order.push("cancel")
			return { kind: "cancelled", commitment: "0xabc" as HexString }
		}
		const { service } = makeService(client)
		const created = await service.create(REQUEST)
		await service.settleFill(created.order.id, 500_000n * ONE)

		expect(order).toEqual(["submit", "cancel", "submit"])
	})

	it("reposts even when the orderbook no longer knows the old entry", async () => {
		const { settled } = await fill(500_000n * ONE, [
			{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" },
		])
		expect(settled.status).toBe("open")
		expect(settled.commitment).toBe("0xabc")
	})

	it("does nothing for an order it does not know", async () => {
		const { service } = makeService(fakeClient([]))
		expect(await service.settleFill("missing", ONE)).toBeNull()
	})

	it("floors the draw-down at zero when a fill delivered more than was left", async () => {
		const { settled } = await fill(9_000_000n * ONE)
		expect(settled.remaining).toBe("0")
		expect(settled.status).toBe("filled")
	})
})

describe("reposting when the old entry will not come down", () => {
	it("leaves the posting alone rather than adding a second entry", async () => {
		// Cancel then post is the right order, but only if the cancel worked. A
		// refusal leaves the old entry live, and posting over it is how two entries
		// end up behind one liability.
		const client = fakeClient([], [{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)

		const reposted = await service.repost(created.order)
		expect(reposted?.commitment).toBe(created.order.commitment)
		expect(reposted?.lastError).toMatch(/SOLVER_MISMATCH/)
		expect(client.submitted).toEqual(["0x00"])
		expect((await store.get(created.order.id))?.status).toBe("open")
	})

	it("goes ahead when the orderbook says the entry is already gone", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		await service.repost(created.order)
		expect(client.submitted).toEqual(["0x00", "0x01"])
	})

	it("holds off when the cancel never got an answer", async () => {
		const client = fakeClient([])
		client.cancelOrder = async () => ({ kind: "failed", message: "connect ECONNREFUSED" })
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const reposted = await service.repost(created.order)
		expect(reposted?.lastError).toMatch(/ECONNREFUSED/)
		expect(client.submitted).toEqual(["0x00"])
	})
})

describe("a posting that lands after the order moved on", () => {
	// A posting is a slow round trip. An operator's cancel, or the expiry sweep,
	// can land while one is in flight, and writing the answer back as `open` would
	// undo it and leave the order matching swaps again.
	const raced = async (status: "cancelled" | "expired") => {
		const client = fakeClient([], [])
		const cancelled: string[] = []
		const inner = client.cancelOrder
		client.cancelOrder = async (params) => {
			cancelled.push(params.commitment)
			return inner(params)
		}
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		await store.setStatus(created.order.id, status)

		// biome-ignore lint/suspicious/noExplicitAny: the posting path is private
		const posted = await (service as any).post({ ...created.order, commitment: null, orderNonce: "1" })
		return { order: posted.order as typeof created.order, store, cancelled, id: created.order.id }
	}

	it("is withdrawn rather than written over a cancel", async () => {
		const { order, store, cancelled, id } = await raced("cancelled")

		expect(order.status).toBe("cancelled")
		expect((await store.get(id))?.status).toBe("cancelled")
		expect(cancelled).toEqual(["0xabc"])
	})

	it("is withdrawn rather than written over an expiry", async () => {
		const { order, cancelled } = await raced("expired")

		expect(order.status).toBe("expired")
		expect(cancelled).toEqual(["0xabc"])
	})

	it("leaves a refusal off a row that already moved on", async () => {
		const client = fakeClient([{ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		await store.setStatus(created.order.id, "cancelled")

		// biome-ignore lint/suspicious/noExplicitAny: the posting path is private
		await (service as any).post({ ...created.order, commitment: null, orderNonce: "1" })
		const order = await store.get(created.order.id)
		expect(order?.status).toBe("cancelled")
		expect(order?.lastError).toBeNull()
	})
})

describe("LimitOrderService.cancel", () => {
	it("cancels locally first, then clears the orderbook entry", async () => {
		const { service, store } = makeService(fakeClient([]))
		const created = await service.create(REQUEST)

		const { order, result } = await service.cancel(created.order.id)
		expect(result.kind).toBe("cancelled")
		expect(order.status).toBe("cancelled")
		expect(order.commitment).toBeNull()
		expect((await store.get(order.id))?.status).toBe("cancelled")
	})

	it("treats an entry the orderbook no longer knows as already cancelled", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { order } = await service.cancel(created.order.id)
		expect(order.status).toBe("cancelled")
		expect(order.commitment).toBeNull()
		expect(order.lastError).toBeNull()
	})

	it("re-signs once on a timestamp the server would not take", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "SIGNATURE_REUSED", message: "same second" }])
		let cancels = 0
		const inner = client.cancelOrder
		client.cancelOrder = async (params) => {
			cancels += 1
			return inner(params)
		}
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { result } = await service.cancel(created.order.id)
		expect(cancels).toBe(2)
		expect(result.kind).toBe("cancelled")
	})

	it("stays cancelled locally even when the orderbook refuses, with the refusal on the row", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { order } = await service.cancel(created.order.id)
		expect(order.status).toBe("cancelled")
		expect(order.lastError).toBe("SOLVER_MISMATCH: wrong key")
	})

	it("refuses an id it does not know", async () => {
		const { service } = makeService(fakeClient([]))
		await expect(service.cancel("nope")).rejects.toThrow(LimitOrderValidationError)
	})
})
