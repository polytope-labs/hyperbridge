-- The last log each processor (one source on one network, one event) committed.
CREATE TABLE indexer.cursors (
    processor text PRIMARY KEY,
    block_number bigint NOT NULL,
    log_index bigint NOT NULL
);
