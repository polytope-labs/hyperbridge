import { describe, expect, it } from "vitest"
import type { AdminStrategyDto } from "@/services/server/dto"
import { buildSolverLink, planSolverLink } from "../../ui/src/lib/solver-link"

const SOLVER = "0x21426D68a9E5Df153FE75cE0fEd20173EBcb80eF"
const market: AdminStrategyDto = {
	index: 0,
	exotic: "USDC/CNGN",
	token0: "USDC",
	token1: "CNGN",
	pricingMode: "static",
	sameToken: false,
	referenceOnly: false,
	maxOrderSize: "5000",
	bid: [{ amount: "1", price: "1373" }],
	ask: [{ amount: "1", price: "1372" }],
}

describe("HyperFX solver links", () => {
	it("encodes the fields the app's parser reads", () => {
		const url = new URL(
			buildSolverLink({ chainId: 8453, from: "usdc", to: "cngn", rate: "1372", reverseRate: "1373", solverAddress: SOLVER, solverName: "Simplex" }),
		)
		expect(url.origin + url.pathname).toBe("https://app.hyperfx.finance/swap")
		expect(Object.fromEntries(url.searchParams)).toEqual({
			wl: "1",
			wlv: "1",
			source: "8453",
			destination: "8453",
			from: "USDC",
			to: "CNGN",
			rate_base: "USDC",
			rate_quote: "CNGN",
			rate: "1372",
			reverse_rate: "1373",
			solver: SOLVER,
			solver_name: "Simplex",
		})
	})

	it("quotes the ask as the rate and the bid as the reverse for a two-sided market", () => {
		const plan = planSolverLink(market, 8453, SOLVER, "A name that is definitely longer than twenty-four chars")
		expect(plan).toMatchObject({ ok: true, from: "USDC", to: "CNGN", rate: "1372", reverseRate: "1373" })
		if (!plan.ok) throw new Error(plan.reason)
		expect(new URL(plan.link).searchParams.get("solver_name")).toBe("A name that is definitel")
	})

	it("flips the direction for a bid-only market and refuses what the app cannot lock", () => {
		const bidOnly = planSolverLink({ ...market, ask: undefined }, 8453, SOLVER, "Simplex")
		expect(bidOnly).toMatchObject({ ok: true, from: "CNGN", to: "USDC", rate: "0.000728332" })
		expect(planSolverLink({ ...market, sameToken: true }, 8453, SOLVER, "Simplex")).toMatchObject({ ok: false })
		expect(planSolverLink({ ...market, pricingMode: "venue" }, 8453, SOLVER, "Simplex")).toMatchObject({ ok: false })
		expect(planSolverLink(market, 8453, "not-an-address", "Simplex")).toMatchObject({ ok: false })
	})
})
