-- The governance forum's tables, filled by a governance config from substrate-eth-rpc's
-- `governance` logs, with the names and columns the governance API reads. Addresses are SS58
-- and JSON columns hold polkadot.js shaped values, as text.
CREATE SCHEMA governance;

CREATE TABLE governance.governance_referendums (
    id text PRIMARY KEY,
    index integer NOT NULL,
    track integer NOT NULL,
    proposer text,
    proposal text NOT NULL,
    proposal_hash text,
    call text,
    state text NOT NULL,
    is_final boolean NOT NULL,
    ayes numeric NOT NULL,
    nays numeric NOT NULL,
    support numeric NOT NULL,
    electorate numeric,
    info text,
    enactment_task text,
    enactment_result text,
    submitted_block integer NOT NULL,
    submitted_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL,
    track_info text
);

CREATE INDEX governance_referendums_track ON governance.governance_referendums (track);
CREATE INDEX governance_referendums_proposer ON governance.governance_referendums (proposer);
CREATE INDEX governance_referendums_proposal_hash
    ON governance.governance_referendums (proposal_hash);
CREATE INDEX governance_referendums_enactment_task
    ON governance.governance_referendums (enactment_task);
CREATE INDEX governance_referendums_ongoing ON governance.governance_referendums (index)
    WHERE NOT is_final;

-- Each track's definition, which only a runtime upgrade changes.
CREATE TABLE governance.governance_tracks (
    id text PRIMARY KEY,
    name text NOT NULL,
    info text NOT NULL,
    updated_block integer NOT NULL
);

-- A referendum's history, including the scheduler dispatch of its enactment.
CREATE TABLE governance.governance_referendum_events (
    id text PRIMARY KEY,
    referendum_id text NOT NULL,
    name text NOT NULL,
    args text,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL,
    changes_state boolean
);

CREATE INDEX governance_referendum_events_referendum
    ON governance.governance_referendum_events (referendum_id, block_number);

CREATE TABLE governance.governance_tally_snapshots (
    id text PRIMARY KEY,
    referendum_id text NOT NULL,
    ayes numeric NOT NULL,
    nays numeric NOT NULL,
    support numeric NOT NULL,
    electorate numeric NOT NULL,
    block_number integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);

CREATE INDEX governance_tally_snapshots_referendum
    ON governance.governance_tally_snapshots (referendum_id, block_number);

-- An account's direct vote on a referendum. Votes on finished referenda keep their final state.
CREATE TABLE governance.governance_votes (
    id text PRIMARY KEY,
    referendum_index integer NOT NULL,
    track integer NOT NULL,
    account text NOT NULL,
    kind text NOT NULL,
    aye boolean,
    conviction integer,
    balance numeric NOT NULL,
    aye_balance numeric,
    nay_balance numeric,
    abstain_balance numeric,
    delegated_votes numeric NOT NULL,
    delegated_capital numeric NOT NULL,
    block_number integer NOT NULL
);

CREATE INDEX governance_votes_referendum ON governance.governance_votes (referendum_index, account);
CREATE INDEX governance_votes_account ON governance.governance_votes (account);

CREATE TABLE governance.governance_vote_calls (
    id text PRIMARY KEY,
    account text NOT NULL,
    referendum_index integer NOT NULL,
    vote text,
    removed boolean NOT NULL,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);

CREATE INDEX governance_vote_calls_referendum
    ON governance.governance_vote_calls (referendum_index, block_number);
CREATE INDEX governance_vote_calls_account ON governance.governance_vote_calls (account);

CREATE TABLE governance.governance_delegations (
    id text PRIMARY KEY,
    delegator text NOT NULL,
    target text NOT NULL,
    track integer NOT NULL,
    conviction integer NOT NULL,
    balance numeric NOT NULL,
    active boolean NOT NULL,
    from_block integer NOT NULL,
    to_block integer
);

CREATE INDEX governance_delegations_delegator
    ON governance.governance_delegations (delegator, track) WHERE active;
CREATE INDEX governance_delegations_target ON governance.governance_delegations (target, track);

-- Technical committee motions.
CREATE TABLE governance.governance_motions (
    id text PRIMARY KEY,
    index integer NOT NULL,
    hash text NOT NULL,
    proposer text NOT NULL,
    threshold integer NOT NULL,
    call text,
    ayes jsonb NOT NULL,
    nays jsonb NOT NULL,
    "end" integer,
    state text NOT NULL,
    is_final boolean NOT NULL,
    result text,
    proposed_block integer NOT NULL,
    proposed_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL
);

CREATE INDEX governance_motions_hash ON governance.governance_motions (hash) WHERE NOT is_final;

CREATE TABLE governance.governance_motion_events (
    id text PRIMARY KEY,
    motion_id text NOT NULL,
    name text NOT NULL,
    args text,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);

CREATE INDEX governance_motion_events_motion
    ON governance.governance_motion_events (motion_id, block_number);

CREATE TABLE governance.governance_whitelisted_calls (
    id text PRIMARY KEY,
    whitelisted_block integer NOT NULL,
    removed_block integer,
    dispatched_block integer,
    dispatch_result text
);

CREATE TABLE governance.governance_preimages (
    id text PRIMARY KEY,
    length integer,
    data text,
    status text NOT NULL,
    noted_block integer,
    updated_block integer NOT NULL
);

CREATE TABLE governance.governance_treasury_spends (
    id text PRIMARY KEY,
    index integer NOT NULL,
    asset_kind text NOT NULL,
    amount numeric NOT NULL,
    beneficiary text NOT NULL,
    valid_from integer NOT NULL,
    expire_at integer NOT NULL,
    state text NOT NULL,
    payment_id text,
    approved_block integer NOT NULL,
    approved_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL
);
