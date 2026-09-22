/**
 * One-line "why" explanations shown before each prompt. Sourced from
 * docs/content/developers/evm/intent-gateway/simplex.mdx — keep the two in sync.
 */
export const WHY = {
	chains: "Simplex listens for orders and fills only on the chains you pick. Each chain needs its own RPC, an ERC-4337 bundler, and funded balances (native gas + stablecoins).",
	rpc: "The RPC is used to scan order events, read balances and simulate fills. Use a premium endpoint with archive access (Alchemy, Infura, QuickNode) — free tiers rate-limit and break event scanning.",
	quorum: "Listing a second, organisationally independent RPC enables quorum log scanning: every event batch must match across providers, so one lying or compromised RPC can't feed you fake orders.",
	bundler:
		"Fills execute as ERC-4337 UserOperations; the bundler submits them on-chain. Alchemy RPC endpoints double as bundlers, or use a dedicated provider like Pimlico.",
	signer: "This wallet signs every fill and holds your stablecoin float on each chain. It's the identity of your filler.",
	substrateKey:
		"Solver-selection orders are won by submitting signed bids to Hyperbridge. This Substrate account signs those bid extrinsics and must hold BRIDGE tokens for fees — the fees are claimed back automatically after fills.",
	hyperbridgeWs: "WebSocket endpoint of the Hyperbridge chain, used to submit and track solver bids.",
	pairs: "Every market you serve is a pair of assets. Same asset on both sides (USDC/USDC) fills cross-chain transfers — ask prices sit just below 1 and the gap is your spread; same-chain same-asset swaps are never filled. Different assets (USDC/CNGN, USDC/USDT, ZARP/CNGN) market-make both directions with your bid/ask curves, same-chain and cross-chain. Assets are referenced by symbol; addresses come from the built-in registry.",
	confirmations:
		"Blocks to wait before filling a cross-chain order, scaled by order value — protects you from reorgs unwinding the deposit after you've paid out.",
	concurrency: "How many orders are processed at once. Lower it if your RPCs rate-limit (429s).",
	gasFeeBump:
		"Percentages added on top of the base gas price for your fill UserOperations. Higher values win more fill races but cost more gas.",
	overfill:
		"Safety clamp against pricing bugs: output is capped at maxOverfillBps above what the user asked for, and the strategy halts after maxConsecutiveClamps consecutive clamped orders.",
	vault: "ERC-4626 treasury (e.g. Aave stataUSDC): fills pull missing balance from the vault atomically, and idle wallet balance above a threshold is swept in to earn yield.",
	allowlist: "Restricts filling to orders placed by specific user addresses. Leave off to fill for everyone.",
	logging: "Log verbosity. 'info' for normal operation, 'debug' when troubleshooting.",
} as const

export const FUNDING_CHECKLIST = [
	"Fund the filler wallet on every selected chain: native token for gas + stablecoins to fill with (docs suggest ~$10k per chain to start).",
	"Keep at least 1 USDC or USDT on each chain for the Simplex gas paymaster. A chain whose fee token has no EIP-2612 permit (BNB Chain) also needs native dust once, to approve that token to Permit2.",
	"Fund the Substrate account with BRIDGE tokens for bid fees (claimed back automatically).",
	"Use premium RPC endpoints with archive access; free tiers will rate-limit.",
].join("\n")
