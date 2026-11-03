-- Fill points waiting for their order. The destination chain values a fill when it indexes it,
-- with its own tokens; when the order isn't indexed yet, the points wait here until it is.
CREATE TABLE indexer.deferred_fill_points (
    id text PRIMARY KEY,
    commitment text NOT NULL,
    filler text NOT NULL,
    usd text NOT NULL,
    transaction_hash text NOT NULL,
    timestamp bigint NOT NULL
);

CREATE INDEX deferred_fill_points_commitment ON indexer.deferred_fill_points (commitment);
