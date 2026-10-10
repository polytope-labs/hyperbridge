/**
 * One-line "why" explanations shown before each prompt. Sourced from
 * docs/content/developers/evm/simplex/ — keep the two in sync.
 */
export const WHY = {
	chains: "Simplex listens for orders and fills only on the chains you pick. Each chain needs its own RPC and funded balances (native gas + stablecoins); fills go through Hyperbridge's bundler.",
	rpc: "The RPC is used to scan order events, read balances and simulate fills. Use a premium endpoint with archive access (Alchemy, Infura, QuickNode) — free tiers rate-limit and break event scanning.",
	quorum: "Listing a second, organisationally independent RPC enables quorum log scanning: every event batch must match across providers, so one lying or compromised RPC can't feed you fake orders.",
	signer: "This wallet signs every fill and holds your stablecoin float on each chain. It's the identity of your filler.",
	secretPhrase:
		"A secret phrase is a BIP-39 mnemonic of 12 to 24 English words. The filler signs with the wallet at m/44'/60'/0'/0/0 unless you pick another account index. The phrase is written to the config file in plain text, protected only by the file's permissions, the same as a private key.",
	substrateKey:
		"Solver-selection orders are won by submitting signed bids to Hyperbridge. This Substrate account signs those bid extrinsics and must hold BRIDGE tokens for fees — the fees are claimed back automatically after fills.",
	hyperbridgeWs: "WebSocket endpoint of the Hyperbridge chain, used to submit and track solver bids.",
	pairs: "Markets name the asset pairs the dashboard lists (USDC/CNGN, USDT/CNGN). They carry no prices: simplex fills only against the limit orders you post from the dashboard once it is running, each stating what it takes in, what it pays out, and on which chain. Assets are referenced by symbol; addresses come from the built-in registry.",
	confirmations:
		"Blocks to wait before filling a cross-chain order, scaled by order value — protects you from reorgs unwinding the deposit after you've paid out.",
	concurrency: "How many orders are processed at once. Lower it if your RPCs rate-limit (429s).",
	gasFeeBump:
		"Percentages added on top of the base gas price for your fill UserOperations. Higher values win more fill races but cost more gas.",
	overfill:
		"A warning, not a clamp: simplex logs a limit order whose offer exceeds what the swapper asked for by more than maxOverfillBps, which usually means the order is mispriced. maxConsecutiveClamps is kept for older configs and no longer halts anything.",
	vault: "ERC-4626 treasury (e.g. Aave stataUSDC): fills pull missing balance from the vault atomically, and idle wallet balance above a threshold is swept in to earn yield.",
	allowlist: "Restricts filling to orders placed by specific user addresses. Leave off to fill for everyone.",
	logging: "Log verbosity. 'info' for normal operation, 'debug' when troubleshooting.",
} as const

export const FUNDING_CHECKLIST = [
	"Fund the filler wallet on every selected chain: native token for gas + stablecoins to fill with (docs suggest ~$10k per chain to start).",
	"Keep at least 1 USDC or USDT on each chain for the Simplex gas paymaster. A chain whose fee token has no EIP-2612 permit (BNB Chain) also needs native dust once, to approve that token to Permit2.",
	"Fund the Substrate account with BRIDGE tokens for bid fees (claimed back automatically).",
	"Use premium RPC endpoints with archive access; free tiers will rate-limit.",
	"Once the solver is running, post limit orders from the dashboard: simplex fills nothing until one is open.",
].join("\n")
