CREATE SCHEMA IF NOT EXISTS forum;

-- Milliseconds since the epoch, which is how the frontend reads block times. The indexer stores
-- timestamps without a zone, in UTC.
CREATE OR REPLACE FUNCTION forum.ms(t TIMESTAMPTZ) RETURNS BIGINT IMMUTABLE LANGUAGE SQL
	AS $$ SELECT (extract(epoch FROM t) * 1000)::bigint $$;
CREATE OR REPLACE FUNCTION forum.ms(t TIMESTAMP) RETURNS BIGINT IMMUTABLE LANGUAGE SQL
	AS $$ SELECT (extract(epoch FROM t AT TIME ZONE 'UTC') * 1000)::bigint $$;

-- Off-chain content for a discussion or an on-chain item. On-chain items are keyed by their index,
-- discussions by a number drawn from the sequence below.
CREATE TABLE IF NOT EXISTS forum.posts (
	kind SMALLINT NOT NULL,
	number BIGINT NOT NULL,
	title TEXT,
	content TEXT,
	content_type TEXT NOT NULL DEFAULT 'markdown',
	author TEXT,
	labels JSONB NOT NULL DEFAULT '[]',
	banner_cid TEXT,
	hidden BOOLEAN NOT NULL DEFAULT FALSE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	PRIMARY KEY (kind, number)
);

CREATE SEQUENCE IF NOT EXISTS forum.discussion_number START 1;

CREATE TABLE IF NOT EXISTS forum.comments (
	id BIGSERIAL PRIMARY KEY,
	post_kind SMALLINT NOT NULL,
	post_number BIGINT NOT NULL,
	reply_to BIGINT REFERENCES forum.comments (id),
	author TEXT NOT NULL,
	content TEXT NOT NULL,
	content_type TEXT NOT NULL DEFAULT 'markdown',
	deleted BOOLEAN NOT NULL DEFAULT FALSE,
	-- The comment's id on the forum it was imported from
	imported_from TEXT UNIQUE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS comments_post ON forum.comments (post_kind, post_number);

-- Reactions on posts and comments, where target is the post or comment id as the API exposes it
CREATE TABLE IF NOT EXISTS forum.reactions (
	target TEXT NOT NULL,
	address TEXT NOT NULL,
	reaction SMALLINT NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	PRIMARY KEY (target, address)
);

-- Images used in posts and banners, keyed by a hash of their content
CREATE TABLE IF NOT EXISTS forum.files (
	id TEXT PRIMARY KEY,
	content_type TEXT NOT NULL,
	data BYTEA NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- What an account says about itself as someone others can delegate their votes to
CREATE TABLE IF NOT EXISTS forum.delegate_announcements (
	address TEXT PRIMARY KEY,
	short_description TEXT NOT NULL,
	long_description TEXT NOT NULL,
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Notes a proposer adds to their referendum after submitting it
CREATE TABLE IF NOT EXISTS forum.appendants (
	id BIGSERIAL PRIMARY KEY,
	referendum_index INT NOT NULL,
	author TEXT NOT NULL,
	content TEXT NOT NULL,
	content_type TEXT NOT NULL DEFAULT 'markdown',
	deleted BOOLEAN NOT NULL DEFAULT FALSE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS forum.users (
	address TEXT PRIMARY KEY,
	avatar_cid TEXT,
	banner_cid TEXT,
	bio TEXT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS forum.login_attempts (
	id TEXT PRIMARY KEY,
	address TEXT NOT NULL,
	challenge TEXT NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Expired challenges are cleared whenever a new one is made
CREATE INDEX IF NOT EXISTS login_attempts_created ON forum.login_attempts (created_at);

-- Sessions are found by the sha256 of their token, as hex, so the table holds no usable token
CREATE TABLE IF NOT EXISTS forum.sessions (
	token_hash TEXT PRIMARY KEY,
	address TEXT NOT NULL,
	expires_at TIMESTAMPTZ NOT NULL
);

-- Databases from before hashing keep their sessions, with each token replaced by its hash
DO $$ BEGIN
	IF EXISTS (SELECT FROM information_schema.columns
		WHERE table_schema = 'forum' AND table_name = 'sessions' AND column_name = 'token') THEN
		UPDATE forum.sessions SET token = encode(sha256(convert_to(token, 'UTF8')), 'hex');
		ALTER TABLE forum.sessions RENAME COLUMN token TO token_hash;
	END IF;
END $$;

CREATE INDEX IF NOT EXISTS sessions_expires ON forum.sessions (expires_at);

-- Wallet signed requests already acted on, so each is accepted once while it is still valid.
-- The hash covers the signing account and the signed entity.
CREATE TABLE IF NOT EXISTS forum.used_signatures (
	hash BYTEA PRIMARY KEY,
	expires_at TIMESTAMPTZ NOT NULL
);

-- Comments an admin marked as spam; the frontend hides them by default
ALTER TABLE forum.comments ADD COLUMN IF NOT EXISTS spam BOOLEAN NOT NULL DEFAULT FALSE;

-- How a referendum's proposer or an admin wants its comments filtered
ALTER TABLE forum.posts ADD COLUMN IF NOT EXISTS comment_filter JSONB;

-- Whether the post was edited after it was written. The description of an on-chain item counts as
-- edited from its first write, since the item existed before it.
ALTER TABLE forum.posts ADD COLUMN IF NOT EXISTS edited BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS forum.reports (
	id BIGSERIAL PRIMARY KEY,
	post_type TEXT NOT NULL,
	post_id TEXT NOT NULL,
	reason TEXT NOT NULL,
	reporter TEXT NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Multisig accounts an account saved, so it can sign for them
CREATE TABLE IF NOT EXISTS forum.multisigs (
	owner TEXT NOT NULL,
	multisig_address TEXT NOT NULL,
	name TEXT NOT NULL,
	threshold SMALLINT NOT NULL,
	signatories JSONB NOT NULL,
	PRIMARY KEY (owner, multisig_address)
);

-- A post's replies, and an account's comments for its profile
CREATE INDEX IF NOT EXISTS comments_reply_to ON forum.comments (reply_to, id) WHERE reply_to IS NOT NULL;
CREATE INDEX IF NOT EXISTS comments_author ON forum.comments (author, id DESC) WHERE NOT deleted;

-- Who uploaded a file and how large it is, so each account's uploads fit a daily quota. Imported
-- files have no uploader.
ALTER TABLE forum.files ADD COLUMN IF NOT EXISTS uploader TEXT;
ALTER TABLE forum.files ADD COLUMN IF NOT EXISTS size INT;
CREATE INDEX IF NOT EXISTS files_uploader ON forum.files (uploader, created_at) WHERE uploader IS NOT NULL;

-- The votes a balance carries with its conviction: a tenth without a lock, then one to six times
CREATE OR REPLACE FUNCTION forum.conviction_votes(balance NUMERIC, conviction INT) RETURNS NUMERIC
	IMMUTABLE LANGUAGE SQL
	AS $$ SELECT CASE WHEN conviction = 0 THEN div(balance, 10) ELSE balance * conviction END $$;
