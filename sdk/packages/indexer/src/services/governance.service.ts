import { SubstrateEvent } from "@subql/types"
import { Option } from "@polkadot/types"
import { Call } from "@polkadot/types/interfaces"
import type { Codec } from "@polkadot/types/types"
import { bnToU8a, compactAddLength, stringToU8a, u8aConcat, u8aToHex } from "@polkadot/util"
import { blake2AsHex } from "@polkadot/util-crypto"
import {
	GovernanceDelegation,
	GovernanceMotion,
	GovernanceMotionEvent,
	GovernancePreimage,
	GovernanceReferendum,
	GovernanceReferendumEvent,
	GovernanceTallySnapshot,
	GovernanceTrack,
	GovernanceTreasurySpend,
	GovernanceVote,
	GovernanceVoteCall,
	GovernanceWhitelistedCall,
} from "@/configs/src/types"
import type {
	AccountVote,
	CollectiveVotes,
	PreimageRequestStatus,
	ReferendumInfo as ReferendumInfoValue,
	ReferendumTally,
	Tracks,
	Voting,
} from "@/types/governance"
import { getSubstrateBlockTimestamp } from "@/utils/rpc.helpers"

type ReferendumInfo = Option<ReferendumInfoValue>

interface EventPosition {
	id: string
	blockNumber: number
	blockHash: string
	extrinsicIndex?: number
	eventIndex: number
	timestamp: Date
}

/**
 * Governance indexing for the Nexus OpenGov, technical committee, whitelist, preimage and treasury
 * pallets. SubQuery pins the global `api` to the block being indexed, so every storage read here sees
 * the state right after the event's block and the stored history stays exact.
 */
export class GovernanceService {
	static async position(event: SubstrateEvent): Promise<EventPosition> {
		const blockNumber = event.block.block.header.number.toNumber()
		const blockHash = event.block.block.header.hash.toString()
		return {
			id: `${blockNumber}-${event.idx}`,
			blockNumber,
			blockHash,
			extrinsicIndex: event.phase.isApplyExtrinsic ? event.phase.asApplyExtrinsic.toNumber() : undefined,
			eventIndex: event.idx,
			// Read from storage since the node skips decoding the timestamp extrinsic
			timestamp: new Date(Number(await getSubstrateBlockTimestamp(blockHash))),
		}
	}

	/** Event fields keyed by name, as JSON, for the timeline. */
	static args(event: SubstrateEvent): Record<string, unknown> {
		const { data } = event.event
		return Object.fromEntries((data.names ?? []).map((name, i) => [name, data[i].toJSON()]))
	}

	/**
	 * The name pallet-referenda gives the scheduled enactment of a referendum, so the scheduler's
	 * `Dispatched` event can be traced back to it.
	 */
	static enactmentTask(index: number): string {
		const encoded = u8aConcat(
			stringToU8a("assembly"),
			compactAddLength(stringToU8a("enactment")),
			bnToU8a(index, { bitLength: 32 }),
		)
		return blake2AsHex(encoded, 256)
	}

	static tally(info: ReferendumInfo): ReferendumTally | undefined {
		if (info.isNone || !info.unwrap().isOngoing) return undefined
		const { ayes, nays, support } = info.unwrap().asOngoing.tally
		return { ayes: ayes.toBigInt(), nays: nays.toBigInt(), support: support.toBigInt() }
	}

	/** The referendum's details while it is live, kept once it ends since storage then only holds the outcome. */
	static ongoingInfo(info: ReferendumInfo): string | undefined {
		if (info.isNone || !info.unwrap().isOngoing) return undefined
		return JSON.stringify(info.unwrap().asOngoing.toJSON())
	}

	static async handleReferendaEvent(event: SubstrateEvent): Promise<void> {
		const position = await this.position(event)
		const args = this.args(event)
		const index = Number(args.index)
		const info = await api.query.referenda.referendumInfoFor<ReferendumInfo>(index)

		let referendum = await GovernanceReferendum.get(String(index))
		if (!referendum) {
			if (event.event.method !== "Submitted") {
				logger.warn(`Referendum ${index} has no Submitted event before ${event.event.method}`)
				return
			}
			referendum = GovernanceReferendum.create({
				id: String(index),
				index,
				track: Number(args.track),
				proposer: info.unwrapOr(undefined)?.asOngoing.submissionDeposit.who.toString(),
				proposal: JSON.stringify(args.proposal),
				proposalHash: this.proposalHash(args.proposal),
				call: await this.proposalCall(args.proposal),
				state: "Submitted",
				isFinal: false,
				ayes: 0n,
				nays: 0n,
				support: 0n,
				submittedBlock: position.blockNumber,
				submittedAt: position.timestamp,
				updatedBlock: position.blockNumber,
			})
			await this.syncTracks(position)
		}

		const tally = this.tally(info) ?? this.eventTally(args.tally)
		if (tally) Object.assign(referendum, tally)
		referendum.info = this.ongoingInfo(info) ?? referendum.info
		if (event.event.method === "Confirmed") referendum.enactmentTask = this.enactmentTask(index)
		referendum.state = event.event.method
		referendum.isFinal = info.isNone || !info.unwrap().isOngoing
		referendum.updatedBlock = position.blockNumber

		await referendum.save()
		await this.saveReferendumEvent(referendum.id, event.event.method, args, position)
	}

	/** Refreshes the track definitions, which only change with a runtime upgrade. */
	static async syncTracks(position: EventPosition): Promise<void> {
		for (const [id, details] of api.consts.referenda.tracks as unknown as Tracks) {
			const info = details.toJSON() as { name: string }
			await GovernanceTrack.create({
				id: id.toString(),
				name: info.name.replace(/\0+$/, ""),
				info: JSON.stringify(info),
				updatedBlock: position.blockNumber,
			}).save()
		}
	}

	static async handleSchedulerDispatchedEvent(event: SubstrateEvent): Promise<void> {
		const args = this.args(event)
		if (!args.id) return

		const [referendum] = await GovernanceReferendum.getByFields([["enactmentTask", "=", args.id as string]], {
			limit: 1,
		})
		if (!referendum) return

		const position = await this.position(event)
		referendum.state = "Executed"
		referendum.enactmentResult = JSON.stringify(args.result)
		referendum.updatedBlock = position.blockNumber
		await referendum.save()
		await this.saveReferendumEvent(referendum.id, "Executed", { result: args.result }, position)
	}

	static async handleConvictionVotingEvent(event: SubstrateEvent): Promise<void> {
		const position = await this.position(event)
		const [who, target] = event.event.data.map((value) => value.toString())

		// Undelegated carries no target, so the delegatees are taken from the delegations it ends.
		const previousTargets =
			event.event.method === "Undelegated"
				? (
						await GovernanceDelegation.getByFields(
							[
								["delegator", "=", who],
								["active", "=", true],
							],
							{ limit: 100 },
						)
					).map((delegation) => delegation.target)
				: []
		const accounts = new Set([who, ...(event.event.method === "Delegated" ? [target] : []), ...previousTargets])

		for (const account of accounts) {
			await this.syncVoting(account, position)
		}
	}

	/**
	 * Brings an account's votes and delegations in line with `ConvictionVoting::VotingFor` at this
	 * block. Only ongoing referenda are touched, so votes on finished ones keep their final state.
	 */
	static async syncVoting(account: string, position: EventPosition): Promise<void> {
		const voting = (await api.query.convictionVoting.votingFor.entries(account)).map(([key, value]) => ({
			track: Number(key.args[1].toString()),
			voting: value as Voting,
		}))

		await this.syncDelegations(account, voting, position)

		const casting = new Map<
			number,
			{ track: number; vote: AccountVote; delegations: Voting["asCasting"]["delegations"] }
		>(
			voting
				.filter(({ voting }) => voting.isCasting)
				.flatMap(({ track, voting }) => {
					const { votes, delegations } = voting.asCasting
					return votes.map(([poll, vote]) => [poll.toNumber(), { track, vote, delegations }] as const)
				}),
		)

		const ongoing = await GovernanceReferendum.getByFields([["isFinal", "=", false]], { limit: 100 })
		const changed: number[] = []

		for (const referendum of ongoing) {
			const id = `${referendum.index}-${account}`
			const existing = await GovernanceVote.get(id)
			const current = casting.get(referendum.index)

			if (!current) {
				if (!existing) continue
				await GovernanceVote.remove(id)
				await this.saveVoteCall(account, referendum.index, undefined, true, position)
				changed.push(referendum.index)
				continue
			}

			const vote = GovernanceVote.create({
				id,
				referendumIndex: referendum.index,
				track: current.track,
				account,
				...this.voteFields(current.vote),
				delegatedVotes: current.delegations.votes.toBigInt(),
				delegatedCapital: current.delegations.capital.toBigInt(),
				blockNumber: position.blockNumber,
			})
			const voteChanged = !existing || this.voteKey(existing) !== this.voteKey(vote)
			if (!voteChanged && existing?.delegatedVotes === vote.delegatedVotes) continue

			await vote.save()
			if (voteChanged) await this.saveVoteCall(account, referendum.index, current.vote.toJSON(), false, position)
			changed.push(referendum.index)
		}

		for (const index of changed) {
			await this.saveTallySnapshot(index, position)
		}
	}

	static voteFields(vote: AccountVote) {
		if (vote.isStandard) {
			const { vote: direction, balance } = vote.asStandard
			const byte = direction.toU8a()[0]
			return { kind: "Standard", aye: (byte & 0x80) !== 0, conviction: byte & 0x7f, balance: balance.toBigInt() }
		}
		if (vote.isSplit) {
			const { aye, nay } = vote.asSplit
			return {
				kind: "Split",
				balance: aye.toBigInt() + nay.toBigInt(),
				ayeBalance: aye.toBigInt(),
				nayBalance: nay.toBigInt(),
			}
		}
		const { aye, nay, abstain } = vote.asSplitAbstain
		return {
			kind: "SplitAbstain",
			balance: aye.toBigInt() + nay.toBigInt() + abstain.toBigInt(),
			ayeBalance: aye.toBigInt(),
			nayBalance: nay.toBigInt(),
			abstainBalance: abstain.toBigInt(),
		}
	}

	static voteKey(vote: GovernanceVote): string {
		return [
			vote.kind,
			vote.aye,
			vote.conviction,
			vote.balance,
			vote.ayeBalance,
			vote.nayBalance,
			vote.abstainBalance,
		].join(":")
	}

	static async syncDelegations(
		account: string,
		voting: { track: number; voting: Voting }[],
		position: EventPosition,
	): Promise<void> {
		const delegating = new Map(
			voting
				.filter(({ voting }) => voting.isDelegating)
				.map(({ track, voting }) => {
					const { target, conviction, balance } = voting.asDelegating
					return [
						track,
						{ target: target.toString(), conviction: conviction.index, balance: balance.toBigInt() },
					]
				}),
		)
		const active = await GovernanceDelegation.getByFields(
			[
				["delegator", "=", account],
				["active", "=", true],
			],
			{ limit: 100 },
		)
		const unchanged = new Set(
			active
				.filter((delegation) => {
					const current = delegating.get(delegation.track)
					return (
						current?.target === delegation.target &&
						current.conviction === delegation.conviction &&
						current.balance === delegation.balance
					)
				})
				.map((delegation) => delegation.track),
		)

		for (const delegation of active.filter((delegation) => !unchanged.has(delegation.track))) {
			delegation.active = false
			delegation.toBlock = position.blockNumber
			await delegation.save()
		}

		for (const [track, current] of delegating) {
			if (unchanged.has(track)) continue
			await GovernanceDelegation.create({
				id: `${account}-${track}-${position.blockNumber}`,
				delegator: account,
				track,
				...current,
				active: true,
				fromBlock: position.blockNumber,
			}).save()
		}
	}

	static async saveTallySnapshot(index: number, position: EventPosition): Promise<void> {
		const info = await api.query.referenda.referendumInfoFor<ReferendumInfo>(index)
		const tally = this.tally(info)
		const referendum = await GovernanceReferendum.get(String(index))
		if (!tally || !referendum) return

		Object.assign(referendum, tally)
		referendum.info = this.ongoingInfo(info)
		referendum.updatedBlock = position.blockNumber
		await referendum.save()
		await GovernanceTallySnapshot.create({
			id: `${index}-${position.blockNumber}`,
			referendumId: referendum.id,
			...tally,
			blockNumber: position.blockNumber,
			timestamp: position.timestamp,
		}).save()
	}

	static async handleTechnicalCollectiveEvent(event: SubstrateEvent): Promise<void> {
		const position = await this.position(event)
		const args = this.args(event)
		const method = event.event.method
		const hash = String(args.proposalHash)

		const motion =
			method === "Proposed"
				? await this.newMotion(args, position)
				: (
						await GovernanceMotion.getByFields(
							[
								["hash", "=", hash],
								["isFinal", "=", false],
							],
							{ limit: 1 },
						)
					)[0]
		if (!motion) return

		const voting = await api.query.technicalCollective.voting<Option<CollectiveVotes>>(hash)
		if (voting.isSome) {
			const { ayes, nays, end } = voting.unwrap()
			motion.ayes = ayes.map((account) => account.toString())
			motion.nays = nays.map((account) => account.toString())
			motion.end = end.toNumber()
		}
		if (method !== "Voted" && method !== "Closed") motion.state = method
		if (["Disapproved", "Executed", "Killed"].includes(method)) motion.isFinal = true
		if (method === "Executed") motion.result = JSON.stringify(args.result)
		motion.updatedBlock = position.blockNumber

		await motion.save()
		await GovernanceMotionEvent.create({
			...position,
			motionId: motion.id,
			name: method,
			args: JSON.stringify(args),
		}).save()
	}

	static async newMotion(args: Record<string, unknown>, position: EventPosition): Promise<GovernanceMotion> {
		const hash = String(args.proposalHash)
		const call = await api.query.technicalCollective.proposalOf<Option<Call>>(hash)
		return GovernanceMotion.create({
			id: String(args.proposalIndex),
			index: Number(args.proposalIndex),
			hash,
			proposer: String(args.account),
			threshold: Number(args.threshold),
			call: call.isSome ? JSON.stringify(this.callJson(call.unwrap())) : undefined,
			ayes: [],
			nays: [],
			state: "Proposed",
			isFinal: false,
			proposedBlock: position.blockNumber,
			proposedAt: position.timestamp,
			updatedBlock: position.blockNumber,
		})
	}

	static async handleWhitelistEvent(event: SubstrateEvent): Promise<void> {
		const { blockNumber } = await this.position(event)
		const args = this.args(event)
		const hash = String(args.callHash)
		const call =
			(await GovernanceWhitelistedCall.get(hash)) ??
			GovernanceWhitelistedCall.create({ id: hash, whitelistedBlock: blockNumber })

		if (event.event.method === "CallWhitelisted") call.whitelistedBlock = blockNumber
		if (event.event.method === "WhitelistedCallRemoved") call.removedBlock = blockNumber
		if (event.event.method === "WhitelistedCallDispatched") {
			call.dispatchedBlock = blockNumber
			call.dispatchResult = JSON.stringify(args.result)
		}
		await call.save()
	}

	static async handlePreimageEvent(event: SubstrateEvent): Promise<void> {
		const position = await this.position(event)
		const hash = event.event.data[0].toString()
		const preimage =
			(await GovernancePreimage.get(hash)) ??
			GovernancePreimage.create({ id: hash, status: event.event.method, updatedBlock: position.blockNumber })

		if (event.event.method === "Noted") {
			const noted = await this.preimage(hash)
			preimage.length = noted?.length
			preimage.data = noted?.data.toHex()
			preimage.notedBlock = position.blockNumber

			// A referendum can be submitted before its preimage is noted, so its call is decoded now
			const call = noted && this.decodeCall(noted.data.toU8a(true))
			const waiting = call
				? await GovernanceReferendum.getByFields([["proposalHash", "=", hash]], { limit: 100 })
				: []
			for (const referendum of waiting.filter((referendum) => !referendum.call)) {
				referendum.call = call
				await referendum.save()
			}
		}
		preimage.status = event.event.method
		preimage.updatedBlock = position.blockNumber
		await preimage.save()
	}

	/** A noted preimage's bytes as stored at the current block. */
	static async preimage(hash: string): Promise<{ length: number; data: Codec } | undefined> {
		const status = await api.query.preimage.requestStatusFor<Option<PreimageRequestStatus>>(hash)
		const length = status.isSome ? this.preimageLength(status.unwrap()) : undefined
		if (length === undefined) return undefined
		const data = await api.query.preimage.preimageFor<Option<Codec>>([hash, length])
		return data.isSome ? { length, data: data.unwrap() } : undefined
	}

	/** Decodes a call with the runtime of the current block, or gives undefined if the bytes are not a call. */
	static decodeCall(bytes: Uint8Array | string): string | undefined {
		try {
			return JSON.stringify(this.callJson(api.registry.createType<Call>("Call", bytes)))
		} catch {
			return undefined
		}
	}

	static async proposalCall(proposal: unknown): Promise<string | undefined> {
		const inline = (proposal as { inline?: string }).inline
		if (inline) return this.decodeCall(inline)
		const hash = this.proposalHash(proposal)
		const preimage = hash ? await this.preimage(hash) : undefined
		return preimage && this.decodeCall(preimage.data.toU8a(true))
	}

	static preimageLength(status: PreimageRequestStatus): number | undefined {
		if (status.isUnrequested) return status.asUnrequested.len.toNumber()
		const length = status.asRequested.maybeLen
		return length.isSome ? length.unwrap().toNumber() : undefined
	}

	static async handleTreasurySpendEvent(event: SubstrateEvent): Promise<void> {
		const position = await this.position(event)
		const args = this.args(event)
		const index = Number(args.index)

		if (event.event.method === "AssetSpendApproved") {
			await GovernanceTreasurySpend.create({
				id: String(index),
				index,
				assetKind: JSON.stringify(args.assetKind),
				amount: BigInt(args.amount as string | number),
				beneficiary: JSON.stringify(args.beneficiary),
				validFrom: Number(args.validFrom),
				expireAt: Number(args.expireAt),
				state: "Approved",
				approvedBlock: position.blockNumber,
				approvedAt: position.timestamp,
				updatedBlock: position.blockNumber,
			}).save()
			return
		}

		const spend = await GovernanceTreasurySpend.get(String(index))
		if (!spend) return
		spend.state = {
			Paid: "Paid",
			PaymentFailed: "Failed",
			SpendProcessed: "Processed",
			AssetSpendVoided: "Voided",
		}[event.event.method]!
		if (args.paymentId !== undefined) spend.paymentId = String(args.paymentId)
		spend.updatedBlock = position.blockNumber
		await spend.save()
	}

	static proposalHash(proposal: unknown): string | undefined {
		const value = proposal as { lookup?: { hash: string }; legacy?: { hash: string }; inline?: string }
		return value.lookup?.hash ?? value.legacy?.hash
	}

	static eventTally(tally: unknown): ReferendumTally | undefined {
		if (!tally) return undefined
		const { ayes, nays, support } = tally as Record<keyof ReferendumTally, string | number>
		return { ayes: BigInt(ayes), nays: BigInt(nays), support: BigInt(support) }
	}

	static callJson(call: Call) {
		return {
			callIndex: u8aToHex(call.callIndex),
			section: call.section,
			method: call.method,
			args: call.args.map((value, i) => ({
				name: call.meta.args[i].name.toString(),
				type: call.meta.args[i].type.toString(),
				value: value.toJSON(),
			})),
		}
	}

	static async saveReferendumEvent(
		referendumId: string,
		name: string,
		args: Record<string, unknown>,
		position: EventPosition,
	): Promise<void> {
		await GovernanceReferendumEvent.create({ ...position, referendumId, name, args: JSON.stringify(args) }).save()
	}

	static async saveVoteCall(
		account: string,
		referendumIndex: number,
		vote: unknown,
		removed: boolean,
		position: EventPosition,
	): Promise<void> {
		await GovernanceVoteCall.create({
			id: `${position.id}-${referendumIndex}`,
			account,
			referendumIndex,
			vote: vote === undefined ? undefined : JSON.stringify(vote),
			removed,
			blockNumber: position.blockNumber,
			extrinsicIndex: position.extrinsicIndex,
			timestamp: position.timestamp,
		}).save()
	}
}
