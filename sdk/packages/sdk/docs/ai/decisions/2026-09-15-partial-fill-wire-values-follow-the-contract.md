# 2026-09-15 — `RedeemEscrowPartial` is 6, and `_partialFills` is slot 11

Decided: `RequestKind.RedeemEscrowPartial = 6`, and `PARTIAL_FILLS_SLOT_BIG_ENDIAN` encodes slot 11.
Both mirror `IntentsBase.sol`.

These are wire values, not labels. The request kind is the first byte of every gateway message. The
slot index is part of every storage key a cross-chain cancel proves. Getting either wrong misroutes
a message or proves the wrong storage.

The branch first used 5 and 12. Main has since given 5 to `Execute`, the governance action, so the
new kind takes 6. The compiled layout (`forge inspect`) puts `_partialFills` at 11 and
`_destinationProtocolFees` at 12. The gateway test `testPartialFillsStorageSlotIsEleven` pins the slot
by reading raw storage.

Rejected: reading the slot from the contract at runtime. The gateway has no public getter for it.
