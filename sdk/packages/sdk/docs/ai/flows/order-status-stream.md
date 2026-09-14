# Order status stream

`IntentGateway.orderStatusStream` requires an attached query client and polls `_queryOrderInternal` until it finds an order. The parser maps status metadata and sorts it by ascending timestamp. The stream selects the last status, except that a last `CANCELLED` yields to an existing terminal status. It emits that selection and stops if it is `FILLED`, `REDEEMED`, or `REFUNDED`.

Otherwise the stream polls again, ignores missing responses and unchanged statuses, updates its remembered status before yielding a change, and stops on a terminal status. Cancellation remains intermediate. The separate on-chain `getOrderStatus` path is unchanged.

The query/parser currently have a pre-existing legacy response-shape mismatch; see `../decisions/2026-09-12-cancellation-is-nonterminal.md`. Tests of this flow mock the query response.
