CREATE SCHEMA IF NOT EXISTS forum;

-- Milliseconds since the epoch, which is how SubSquare reports block times. The indexer stores
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
	-- The SubSquare comment id for comments brought over by the import
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

CREATE TABLE IF NOT EXISTS forum.sessions (
	token TEXT PRIMARY KEY,
	address TEXT NOT NULL,
	expires_at TIMESTAMPTZ NOT NULL
);
