# 2026-09-09 — A phantom bid's paymasterAndData is read as a bare declaration first, and as a Permit2 sponsorship with an optional tail second

Chosen: `decodePhantomBidPaymasterAndData` tries the bare declaration parse over the whole blob; only when that
fails does it look for a complete 234-byte Permit2-mode paymasterAndData at the front and parse whatever follows
as the declaration. Both parsers demand exact consumption. A sponsored bid with nothing appended, or with a
malformed tail, is a bid that declared nothing — the absent declaration, still counted.

Alternatives considered:

- Move the declaration out of `paymasterAndData` for sponsored bids (a second field, or a side channel). There is
  no other free signed field: `initCode` is spoken for under EIP-7702, and anything outside the userOpHash is not
  authenticated. The tail is signed for free and the paymaster never parses a phantom bid, so it costs nothing.
- Detect the sponsored shape first. A bare declaration of 234+ bytes whose byte 52 happens to be `0x02` would
  then be tried as a sponsorship and, if its tail did not parse, read as "declared nothing" — a silent change to
  bids that decode today. Bare-first keeps every existing bid byte-identical in behaviour; the reverse false
  positive needs a paymaster address opening with `0x01`/`0x02` AND the gas-limit words and permit bytes forming
  length-prefixed entries that end exactly at the blob's end, which is not a shape any packer produces.
- Recognise the 2612 permit mode (0x00, 150 bytes) as a sponsorship too. Simplex confines that mode to a
  first-time delegation — its sequential nonce serialises concurrent ops — so no bid carries it, and the retired
  allowance mode 0x01 is refused by the paymaster. Accepting only 0x02 keeps the decoder to layouts that exist on
  the wire; a bid in another mode decodes as "none", which is the absent declaration.
- Verify the Permit2 signature, or check its deadline. Neither is a fact about the quote: the bid's authenticity
  is the solver signature over the userOpHash, which covers these bytes, and a phantom bid never executes so an
  expired permit changes nothing. The fields are decoded for the record and the signature is not kept.

The encoder is strict where the decoder is lenient: `encodePhantomBidPaymasterAndData` throws unless the
sponsorship is exactly the Permit2-mode layout, because a solver that signed some other shape would have its
declaration silently read as absent — the failure a lenient decoder cannot report and a strict encoder prevents.
