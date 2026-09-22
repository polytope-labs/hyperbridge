# 2026-09-16 — One limit order through a running orderbook

`orderbook.live.test.ts` takes a limit order through a real HyperFX orderbook: created and accepted,
listed under the solver, heartbeated, resized by a fill, reconciled, then withdrawn. It is skipped
unless the environment says where to find a server:

```
HYPERFX_ORDERBOOK_URL=http://127.0.0.1:8080/graphql   a server already running
HYPERFX_ORDERBOOK_BIN=/path/to/hyperfx-orderbook      a binary to run on a config the test writes
```

Build the binary from polytope-labs/hyperfx-orderbook with
`cargo build --release -p hyperfx-server --bin hyperfx-orderbook`. The test needs no chain: the
config it writes runs the server the way its own `config.dev.toml` does, with the protocol fee a
constant and the validation cycle reading nothing, so an order surfaces at its quoted size with no
balance behind it.

It found two things no other test could.

**`CancelOrder` was signed without the solver.** The server's struct is
`CancelOrder(address solver, bytes32 commitment, uint64 timestamp)`, and we signed the last two, so
every cancel came back `SOLVER_MISMATCH`. Withdrawing a limit order left its entry live, and because
`repost` cancels before it posts, a resize would have put a second entry behind the same liability,
which is the one thing section 6 sets out to avoid. `Heartbeat` already named its solver and was
unaffected.

**`backed: false` does not mean under-funded.** It is also false before any balance cycle has read
the order, and the order surfaces at its full quoted size until one does, so reconciliation was
stamping `UNDER_FUNDED` on healthy postings seconds after they went up. `PostedOrder` now carries
`validatedAt`, and only a `backed: false` a cycle actually decided counts, alongside `resized`.
