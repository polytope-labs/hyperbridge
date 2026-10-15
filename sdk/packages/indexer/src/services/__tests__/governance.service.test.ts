;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
	}),
	getByFields: jest.fn(async (entity: string, filters: [string, string, unknown][], options: any = {}) =>
		[...records.entries()]
			.filter(
				([key, props]) =>
					key.startsWith(`${entity}:`) && filters.every(([field, , value]) => props[field] === value),
			)
			.map(([, props]) => props)
			.slice(0, options.limit),
	),
	remove: jest.fn(async (entity: string, id: string) => records.delete(`${entity}:${id}`)),
}

jest.mock("@/utils/rpc.helpers", () => ({ getSubstrateBlockTimestamp: jest.fn(async () => 1_764_924_258_000n) }))

import { GovernanceService } from "@/services/governance.service"

const balance = (value: bigint) => ({ toBigInt: () => value })
const count = (value: number) => ({ toNumber: () => value })
const account = (value: string) => ({ toString: () => value })

const standard = (aye: boolean, conviction: number, amount: bigint) => ({
	isStandard: true,
	isSplit: false,
	asStandard: { vote: { toU8a: () => Uint8Array.of((aye ? 0x80 : 0) | conviction) }, balance: balance(amount) },
	toJSON: () => ({ standard: { aye, conviction, amount: amount.toString() } }),
})

const split = (aye: bigint, nay: bigint) => ({
	isStandard: false,
	isSplit: true,
	asSplit: { aye: balance(aye), nay: balance(nay) },
	toJSON: () => ({ split: { aye: aye.toString(), nay: nay.toString() } }),
})

const casting = (votes: [number, unknown][], delegatedVotes = 0n) => ({
	isCasting: true,
	isDelegating: false,
	asCasting: {
		votes: votes.map(([poll, vote]) => [count(poll), vote]),
		delegations: { votes: balance(delegatedVotes), capital: balance(delegatedVotes) },
	},
})

const delegating = (target: string, conviction: number, amount: bigint) => ({
	isCasting: false,
	isDelegating: true,
	asDelegating: { target: account(target), conviction: { index: conviction }, balance: balance(amount) },
})

const ongoing = (ayes: bigint) => ({
	isNone: false,
	isSome: true,
	unwrap: () => ({
		isOngoing: true,
		asOngoing: {
			tally: { ayes: balance(ayes), nays: balance(0n), support: balance(ayes) },
			toJSON: () => ({ tally: { ayes: ayes.toString() } }),
		},
	}),
})

// Voting state per account, as `ConvictionVoting::VotingFor` entries keyed by track
const votingFor = new Map<string, [number, unknown][]>()
;(global as any).api = {
	query: {
		convictionVoting: {
			votingFor: {
				entries: jest.fn(async (who: string) =>
					(votingFor.get(who) ?? []).map(([track, voting]) => [
						{ args: [account(who), account(String(track))] },
						voting,
					]),
				),
			},
		},
		referenda: { referendumInfoFor: jest.fn(async () => ongoing(5n)) },
		balances: {
			totalIssuance: jest.fn(async () => balance(100n)),
			inactiveIssuance: jest.fn(async () => balance(40n)),
		},
	},
}

const eventData = (fields: Record<string, unknown>) =>
	Object.assign(
		Object.values(fields).map((value) => ({ toJSON: () => value, toString: () => String(value) })),
		{ names: Object.keys(fields) },
	)

const event = (method: string, fields: Record<string, unknown>, blockNumber = 100) =>
	({
		idx: 3,
		phase: { isApplyExtrinsic: true, asApplyExtrinsic: count(2) },
		event: { method, data: eventData(fields) },
		block: { block: { header: { number: count(blockNumber), hash: account(`0x${blockNumber}`) } } },
	}) as any

const ALICE = "15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY"
const BOB = "12rZBqTSTKFatcEb5BexHB9G2fJVg2hgeCV6U9pYtXtZqHYU"

const referendum = (index: number, isFinal: boolean) =>
	records.set(`GovernanceReferendum:${index}`, {
		id: String(index),
		index,
		track: 1,
		isFinal,
		state: isFinal ? "Confirmed" : "DecisionStarted",
		ayes: 0n,
		nays: 0n,
		support: 0n,
	})

describe("GovernanceService", () => {
	beforeEach(() => {
		records.clear()
		votingFor.clear()
		jest.clearAllMocks()
	})

	it("names the enactment task the way pallet-referenda schedules it", () => {
		// The id of the scheduler Dispatched event that enacted Nexus referendum 15 at block 8069993
		expect(GovernanceService.enactmentTask(15)).toBe(
			"0x000d200ef3a006678d25617d594246894e0102c39c30b04d6e31eb34c6cef151",
		)
	})

	it("reads direction and conviction from a standard vote byte", () => {
		expect(GovernanceService.voteFields(standard(true, 3, 10n) as any)).toMatchObject({
			kind: "Standard",
			aye: true,
			conviction: 3,
			balance: 10n,
		})
		expect(GovernanceService.voteFields(standard(false, 0, 10n) as any)).toMatchObject({
			aye: false,
			conviction: 0,
		})
	})

	it("records a split vote with both sides and a tally snapshot", async () => {
		referendum(1, false)
		votingFor.set(ALICE, [[1, casting([[1, split(7n, 3n)]])]])

		await GovernanceService.handleConvictionVotingEvent(event("Voted", { who: ALICE, vote: {} }))

		expect(records.get(`GovernanceVote:1-${ALICE}`)).toMatchObject({
			kind: "Split",
			balance: 10n,
			ayeBalance: 7n,
			nayBalance: 3n,
		})
		expect(records.get(`GovernanceVoteCall:100-3-1`)).toMatchObject({ account: ALICE, removed: false })
		expect(records.get("GovernanceTallySnapshot:1-100")).toMatchObject({ ayes: 5n, electorate: 60n })
		expect(JSON.parse(records.get("GovernanceReferendum:1").info)).toEqual({ tally: { ayes: "5" } })
	})

	it("removes a vote taken back on an ongoing referendum but keeps votes on finished ones", async () => {
		referendum(1, false)
		referendum(2, true)
		votingFor.set(ALICE, [
			[
				1,
				casting([
					[1, standard(true, 1, 4n)],
					[2, standard(true, 1, 4n)],
				]),
			],
		])
		await GovernanceService.handleConvictionVotingEvent(event("Voted", { who: ALICE, vote: {} }, 100))
		records.set(`GovernanceVote:2-${ALICE}`, { id: `2-${ALICE}`, referendumIndex: 2, account: ALICE })

		votingFor.set(ALICE, [[1, casting([])]])
		await GovernanceService.handleConvictionVotingEvent(event("VoteRemoved", { who: ALICE, vote: {} }, 101))

		expect(records.has(`GovernanceVote:1-${ALICE}`)).toBe(false)
		expect(records.get(`GovernanceVoteCall:101-3-1`)).toMatchObject({ removed: true })
		expect(records.has(`GovernanceVote:2-${ALICE}`)).toBe(true)
	})

	it("opens, replaces and closes delegation periods and refreshes the delegatee", async () => {
		referendum(1, false)
		votingFor.set(BOB, [[1, casting([[1, standard(true, 1, 4n)]], 30n)]])
		votingFor.set(ALICE, [[1, delegating(BOB, 3, 10n)]])

		await GovernanceService.handleConvictionVotingEvent(event("Delegated", { who: ALICE, target: BOB }, 100))

		expect(records.get(`GovernanceDelegation:${ALICE}-1-100`)).toMatchObject({
			target: BOB,
			conviction: 3,
			balance: 10n,
			active: true,
		})
		expect(records.get(`GovernanceVote:1-${BOB}`)).toMatchObject({ delegatedVotes: 30n })

		votingFor.set(ALICE, [[1, delegating(BOB, 6, 10n)]])
		await GovernanceService.handleConvictionVotingEvent(event("Delegated", { who: ALICE, target: BOB }, 110))

		expect(records.get(`GovernanceDelegation:${ALICE}-1-100`)).toMatchObject({ active: false, toBlock: 110 })
		expect(records.get(`GovernanceDelegation:${ALICE}-1-110`)).toMatchObject({ conviction: 6, active: true })

		votingFor.set(ALICE, [])
		votingFor.set(BOB, [[1, casting([[1, standard(true, 1, 4n)]], 0n)]])
		await GovernanceService.handleConvictionVotingEvent(event("Undelegated", { who: ALICE }, 120))

		expect(records.get(`GovernanceDelegation:${ALICE}-1-110`)).toMatchObject({ active: false, toBlock: 120 })
		expect(records.get(`GovernanceVote:1-${BOB}`)).toMatchObject({ delegatedVotes: 0n })
	})

	it("follows a treasury spend from approval to payment", async () => {
		await GovernanceService.handleTreasurySpendEvent(
			event("AssetSpendApproved", {
				index: 0,
				assetKind: { native: null },
				amount: "1000",
				beneficiary: { accountId32: ALICE },
				validFrom: 10,
				expireAt: 20,
			}),
		)
		await GovernanceService.handleTreasurySpendEvent(event("Paid", { index: 0, paymentId: 7 }, 105))

		expect(records.get("GovernanceTreasurySpend:0")).toMatchObject({
			amount: 1000n,
			state: "Paid",
			paymentId: "7",
			approvedBlock: 100,
			updatedBlock: 105,
		})
	})
})
