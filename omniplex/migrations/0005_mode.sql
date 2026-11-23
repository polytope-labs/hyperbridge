-- The modes of the configs that have run on this database, so the indexer and a governance
-- config never share one.
CREATE TABLE indexer.modes (
    mode text PRIMARY KEY
);
