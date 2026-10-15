import type { Option } from "@polkadot/types"
import type { Codec } from "@polkadot/types/types"

// The parts of the governance pallets' storage that the governance indexer reads.

interface Balance extends Codec {
	toBigInt(): bigint
}

interface Count extends Codec {
	toNumber(): number
}

export type Tracks = [Count, Codec][]

export interface ReferendumTally {
	ayes: bigint
	nays: bigint
	support: bigint
}

export interface ReferendumInfo extends Codec {
	isOngoing: boolean
	asOngoing: {
		tally: Record<keyof ReferendumTally, Balance>
		submissionDeposit: { who: Codec }
	}
}

export interface AccountVote extends Codec {
	isStandard: boolean
	// The vote byte packs the direction in its top bit and the conviction in the rest
	asStandard: { vote: Codec; balance: Balance }
	isSplit: boolean
	asSplit: { aye: Balance; nay: Balance }
	asSplitAbstain: { aye: Balance; nay: Balance; abstain: Balance }
}

export interface Voting extends Codec {
	isCasting: boolean
	asCasting: {
		votes: [Count, AccountVote][]
		delegations: { votes: Balance; capital: Balance }
	}
	isDelegating: boolean
	asDelegating: { target: Codec; conviction: { index: number }; balance: Balance }
}

export interface CollectiveVotes extends Codec {
	ayes: Codec[]
	nays: Codec[]
	end: Count
}

export interface PreimageRequestStatus extends Codec {
	isUnrequested: boolean
	asUnrequested: { len: Count }
	asRequested: { maybeLen: Option<Count> }
}
