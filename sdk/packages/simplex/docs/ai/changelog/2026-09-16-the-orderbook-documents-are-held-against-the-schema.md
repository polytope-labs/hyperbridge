# 2026-09-16 — The orderbook documents are held against the published schema

`src/tests/fixtures/orderbook-schema.graphql` is a copy of `schema.graphql` from
polytope-labs/hyperfx-orderbook. Refresh it with

```
gh api repos/polytope-labs/hyperfx-orderbook/contents/schema.graphql -q .content | base64 -d
```

`schema.test.ts` parses every document `OrderbookClient` sends, now exported as `ORDERBOOK_DOCUMENTS`,
and validates it against that schema. Nothing else in the package reads a query: the fake client
answers from a queue, so a renamed field, a mistyped argument or an invalid selection set passes
every other test and fails on the first real request.

It found one. `submitOrder` asked for `code` on both `OrderRejected` and `OrderSubmissionFailed`,
which return `RejectionCode!` and `FailureCode!`. A selection set cannot ask for two enums under one
name, so the server would have refused the whole mutation and every posting would have come back as
a transport failure. `OrderSubmissionFailed.code` is now aliased to `failureCode`.

`RejectionCode`, `MessageRejectionCode` and the new `FailureCode` are arrays with the union derived
from them, so the same test can hold each against the schema's enum. Drift there is otherwise silent:
an unknown code reaches the operator as a string on the row either way, and nothing fails until
someone reads the union and believes it.
