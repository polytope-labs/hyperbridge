# EVM GetRequestTimeoutHandled handler name

The generated EVM chain manifests route `GetRequestTimeoutHandled(bytes32,string)` logs to
`handleGetRequestTimeoutHandledEvent`, but the mapping exported the handler as
`handleGetRequestTimeoutHandled`. SubQuery resolves handlers lazily, so each EVM indexer crashed
with "Handler function ... is not found" on the first GET request timeout it indexed.

The EVM handler is now exported as `handleGetRequestTimeoutHandledEvent`, matching the manifest and
the `*Event` naming of the other EVM host handlers. No reindex is needed: the failing block was never
committed, so indexing resumes from it.
