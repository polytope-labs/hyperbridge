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
	strategies:
		"Strategies decide which orders are profitable to fill. 'stable' fills same-token stablecoin transfers across chains; 'hyperfx' market-makes between stablecoins and one exotic token (e.g. cNGN).",
	bpsCurve:
		"Your profit margin: minimum basis points charged as a function of order size. Points are interpolated into a smooth curve — high bps keeps small orders worthwhile, low bps keeps large orders competitive.",
	maxOrderUsd: "Caps your exposure per order. Larger orders are partially filled up to this cap.",
	token1: "The exotic (non-USD) token the hyperfx strategy trades, per chain it exists on.",
	fxPricing:
		"hyperfx needs a price source for the exotic token: static bid/ask curves you maintain, or a Uniswap V4 LP position whose pool price acts as the oracle (and doubles as liquidity).",
	confirmations:
		"Blocks to wait before filling a cross-chain order, scaled by order value — protects you from reorgs unwinding the deposit after you've paid out.",
	concurrency: "How many orders are processed at once. Lower it if your RPCs rate-limit (429s).",
	gasFeeBump:
		"Percentages added on top of the base gas price for your fill UserOperations. Higher values win more fill races but cost more gas.",
	overfill:
		"Safety clamp against pricing bugs: output is capped at maxOverfillBps above what the user asked for, and the strategy halts after maxConsecutiveClamps consecutive clamped orders.",
	rebalancing:
		"Automatically tops up a chain's stablecoin balance from richer chains when it drops below a fraction of its base level.",
	vault: "ERC-4626 treasury (e.g. Aave stataUSDC): fills pull missing balance from the vault atomically, and idle wallet balance above a threshold is swept in to earn yield.",
	allowlist: "Restricts filling to orders placed by specific user addresses. Leave off to fill for everyone.",
	logging: "Log verbosity. 'info' for normal operation, 'debug' when troubleshooting.",
} as const

export const FUNDING_CHECKLIST = [
	"Fund the filler wallet on every selected chain: native token for gas + stablecoins to fill with (docs suggest ~$10k per chain to start).",
	"Keep at least 1 USDC on each chain for the Circle paymaster (BNB Chain has no paymaster — it needs native BNB).",
	"Fund the Substrate account with BRIDGE tokens for bid fees (claimed back automatically).",
	"Use premium RPC endpoints with archive access; free tiers will rate-limit.",
].join("\n")
