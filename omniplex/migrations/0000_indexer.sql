-- The last log each chain's stream committed.
CREATE TABLE indexer.cursors (
    network text PRIMARY KEY,
    block_number bigint NOT NULL,
    log_index bigint NOT NULL
);
