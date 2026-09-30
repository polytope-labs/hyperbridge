# 2026-09-21 — `FillerBid.bid` is nullable

#1297 added `bid` to `FillerBid` as `String!`. The substrate node's in-place schema migration only adds
nullable fields, so an existing deployment refused the schema on restart:

```
Failed to execute Schema Migration Error: Non-nullable field creation is not supported: bid on FillerBid
```

The field is now `bid: String @index`, and the column is added in place on restart. `handleBidPlaced`
still sets `bid` on every row it writes. Rows indexed before the field existed keep it null.
