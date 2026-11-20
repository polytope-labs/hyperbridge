-- The mode of the config the database belongs to, recorded by its first start.
CREATE TABLE indexer.mode (
    only_row boolean PRIMARY KEY DEFAULT true CHECK (only_row),
    mode text NOT NULL
);
