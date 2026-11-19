-- The block explorer's tables, filled by an explorer config from substrate-eth-rpc's `explorer`
-- logs. Times are milliseconds and addresses are SS58 with prefix 0, as the site shows them.
CREATE SCHEMA explorer;

CREATE TABLE explorer.blocks (
    height bigint PRIMARY KEY,
    hash text NOT NULL,
    parent_hash text NOT NULL,
    state_root text NOT NULL,
    extrinsics_root text NOT NULL,
    time bigint NOT NULL,
    spec_version integer NOT NULL,
    extrinsics_count integer NOT NULL,
    events_count integer NOT NULL
);

CREATE INDEX blocks_hash ON explorer.blocks (hash);

CREATE TABLE explorer.extrinsics (
    height bigint NOT NULL,
    index integer NOT NULL,
    hash text NOT NULL,
    time bigint NOT NULL,
    signed boolean NOT NULL,
    success boolean NOT NULL,
    signer text,
    section text NOT NULL,
    method text NOT NULL,
    args jsonb,
    events_count integer NOT NULL,
    PRIMARY KEY (height, index)
);

CREATE INDEX extrinsics_hash ON explorer.extrinsics (hash);
CREATE INDEX extrinsics_signer ON explorer.extrinsics (signer, height DESC, index);
CREATE INDEX extrinsics_call ON explorer.extrinsics (section, method, height DESC, index);
CREATE INDEX extrinsics_signed ON explorer.extrinsics (height DESC, index) WHERE signed;

CREATE TABLE explorer.events (
    height bigint NOT NULL,
    index integer NOT NULL,
    time bigint NOT NULL,
    extrinsic_index integer,
    section text NOT NULL,
    method text NOT NULL,
    args jsonb,
    PRIMARY KEY (height, index)
);

CREATE INDEX events_call ON explorer.events (section, method, height DESC, index);

CREATE TABLE explorer.transfers (
    height bigint NOT NULL,
    event_index integer NOT NULL,
    time bigint NOT NULL,
    extrinsic_index integer,
    "from" text NOT NULL,
    "to" text NOT NULL,
    amount numeric NOT NULL,
    PRIMARY KEY (height, event_index)
);

CREATE INDEX transfers_from ON explorer.transfers ("from", height DESC, event_index);
CREATE INDEX transfers_to ON explorer.transfers ("to", height DESC, event_index);

-- Each account's `System.Account` as of the last block that touched it.
CREATE TABLE explorer.accounts (
    address text PRIMARY KEY,
    nonce bigint NOT NULL,
    consumers bigint NOT NULL,
    providers bigint NOT NULL,
    sufficients bigint NOT NULL,
    free numeric NOT NULL,
    reserved numeric NOT NULL,
    frozen numeric NOT NULL,
    total numeric NOT NULL,
    height bigint NOT NULL
);

CREATE INDEX accounts_total ON explorer.accounts (total DESC);

-- Every call and event name seen under each runtime, for the site's section and method filters.
CREATE TABLE explorer.call_names (
    spec_version integer NOT NULL,
    kind text NOT NULL,
    section text NOT NULL,
    method text NOT NULL,
    PRIMARY KEY (spec_version, kind, section, method)
);

CREATE TABLE explorer.status (
    name text PRIMARY KEY,
    value text NOT NULL
);
