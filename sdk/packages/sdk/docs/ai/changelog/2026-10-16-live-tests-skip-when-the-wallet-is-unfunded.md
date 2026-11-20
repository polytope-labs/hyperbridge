# Live tests skip, with a warning, when the test wallet cannot pay

The live tests in `src/tests/sequential/` spend testnet funds from one account. Before its first
transaction, each test reads the balances it is about to spend and skips when any is below that
test's minimum. The minimums are named constants in each test file. A transaction the chain rejects
with viem's `InsufficientFundsError`, anywhere in its error's `cause` chain, also skips the test,
because gas prices and relayer fees drift. Any other error still fails the test, and a missing
secret is still a hard error.

Each skip prints a GitHub Actions annotation and the same text to stderr:

```
::warning title=Unfunded live test::<test>: <account> holds <balance> <symbol> on <chain>, needs at least <min> <symbol>. Fund it to run this test.
```

To run a skipped test, fund the named balance. These are the floors:

| Script | Account | Minimums |
|--------|---------|----------|
| `test:intent-gateway-cancel`, from source | `PRIVATE_KEY` | 5 POL, 10 fee token and 0.1 USDC on Polygon Amoy |
| `test:intent-gateway-cancel`, from destination | `PRIVATE_KEY` | 3 POL and 10 fee token on Amoy; 0.05 tBNB, 0.1 fee token and 0.1 USDC on BSC Chapel |
| `test:hyper-fungible-token`, the bridge test | `PRIVATE_KEY` | 0.01 tBNB and 10 fee token on Chapel, read after the faucet top-up |
| `test:get-request` | `PRIVATE_KEY` | 0.05 tBNB on Chapel |
| `test:intents-coprocessor` | `SECRET_PHRASE` | 1 tBRIDGE free on Hyperbridge Gargantua; the bid's later tests skip with it |

A cancel test that skips after placing its order leaves that testnet escrow unrefunded, so its
upfront minimums are set to catch a short wallet before placement. `src/tests/sequential/funding.ts`
holds the check and the backstop; `src/tests/liveTestFunding.test.ts` covers them offline.
