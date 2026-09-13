# Intent gateway order placement enrichment

1. `handleOrderPlacedEventV3` reads the event's block timestamp and order fields. Current events carry the complete order and graffiti. Legacy events recover the missing call data and graffiti through the existing direct-calldata or trace path.
2. The handler computes the commitment and resolves the source host's fee token at the placement block.
3. `resolvePlacementUserOpHash` reads the transaction receipt, finds the next canonical EntryPoint operation event, verifies the sender against the placing account, and checks that the placement is within the bundle's execution phase. Missing transactions, failed receipt lookup, or uncertain attribution return no hash.
4. `getOrCreateOrder` saves fee denomination and the optional placement hash along with the existing order details. For an existing row, an absent hash leaves its stored hash intact. Asset persistence, points, volumes, pending metadata, and status behavior follow the established path.
5. The handler records `PLACED` metadata through `updateOrderStatus` and runs its existing volume update. GraphQL clients can request `IOrderV3.userOpHash` independently of `transactionHash` and the hashes on fill relations.
