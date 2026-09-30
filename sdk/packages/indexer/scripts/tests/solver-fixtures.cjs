// The solvers the solver-inventory E2E seeds, publishes and asserts on. One definition, shared by
// the fake orderbook, the anvil seeding and the assertions, so a fixture can only ever change in
// all three at once.
//
// Addresses are arbitrary but fixed: nothing on a Base fork holds their balances until the seeding
// writes them, which is what makes the assertions exact.

/** Base, the only chain this E2E indexes. */
const CHAIN = "EVM-8453"

/** Native USDC on Base, with the storage slot its balances live in (config-mainnet tokenSlots). */
const USDC = {
	address: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
	balanceSlot: 9,
	decimals: 6,
}

/** stataUSDC, the ERC-4626 vault wrapping USDC on Base; the solvers hold no shares in it. */
const VAULT = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"

/** A SolverAccount from the Base config: delegation counts only when it points at one of these. */
const SOLVER_ACCOUNT = "0xd5535d4deb17f050e52b6efda2fde00435f39279"

/** Not a SolverAccount, so a 7702 designator pointing here is recorded but not counted. */
const FOREIGN_DELEGATE = "0x00000000000000000000000000000000000de1e6"

const usdc = (whole) => BigInt(whole) * 10n ** BigInt(USDC.decimals)

/**
 * One solver per delegation shape, so the indexed `delegated` flag is exercised in all three
 * states rather than only the true one.
 */
const SOLVERS = [
	{
		name: "delegated",
		address: "0x5011e700000000000000000000000000000000a1",
		usdc: usdc(1_500),
		/** EIP-7702 designator pointing at a known SolverAccount. */
		delegateTo: SOLVER_ACCOUNT,
		expect: { delegated: true, delegate: SOLVER_ACCOUNT },
	},
	{
		name: "delegated-elsewhere",
		address: "0x5011e700000000000000000000000000000000b2",
		usdc: usdc(750),
		/** Delegated, but not to a SolverAccount: recorded with the delegate, counted as false. */
		delegateTo: FOREIGN_DELEGATE,
		expect: { delegated: false, delegate: FOREIGN_DELEGATE },
	},
	{
		name: "plain-eoa",
		address: "0x5011e700000000000000000000000000000000c3",
		usdc: usdc(250),
		delegateTo: null,
		expect: { delegated: false, delegate: null },
	},
]

/**
 * The Transfer that proves the rows are event-sourced after their genesis read, not re-read.
 *
 * Sent from the plain EOA: EIP-3607 rejects transactions from an account carrying code, which the
 * two delegated solvers do. Both sides are tracked, so one log has to move two rows in opposite
 * directions.
 */
const TRANSFER = {
	from: SOLVERS[2].address,
	to: SOLVERS[0].address,
	amount: usdc(100),
}

/** What each solver's wallet must hold once the Transfer above has been indexed. */
function expectedWallets() {
	const wallets = new Map(SOLVERS.map((solver) => [solver.address, solver.usdc]))
	wallets.set(TRANSFER.from, wallets.get(TRANSFER.from) - TRANSFER.amount)
	wallets.set(TRANSFER.to, wallets.get(TRANSFER.to) + TRANSFER.amount)
	return wallets
}

module.exports = { CHAIN, USDC, VAULT, SOLVER_ACCOUNT, FOREIGN_DELEGATE, SOLVERS, TRANSFER, expectedWallets }
