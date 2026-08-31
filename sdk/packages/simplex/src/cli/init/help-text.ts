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
	anchor: "Confirmation depth is sized in USD and your curves are the only price feed, so every quote asset must connect to a USD stable through some curve-priced pair. A reference-only pair contributes its rate without opening a market.",
	sameAssetCurve:
		"Ask prices are the fraction of the input you pay back out, by order size. 0.995 keeps 0.5% of every fill; prices at or above 1 can never profit and are rejected.",
	maxOrderSize: "Caps your exposure per order, in units of the pair's quote asset (token0). Larger orders are partially filled up to this cap.",
	crossAssetCurves:
		"Curves price token1 per 1 token0 by order size. The bid curve is what you pay when buying token1 from users; the ask is what you charge selling it. Bid must stay above ask everywhere (uncrossed book). Omit one side for one-sided LP.",
	fxPricing:
		"A cross-asset pair needs a price source: static bid/ask curves you maintain, or a Uniswap V4 LP position whose pool price acts as the oracle (and doubles as liquidity).",
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
	"Keep at least 1 USDC on each chain for the Circle paymaster (BNB Chain has no paymaster — it needs native BNB).",
	"Fund the Substrate account with BRIDGE tokens for bid fees (claimed back automatically).",
	"Use premium RPC endpoints with archive access; free tiers will rate-limit.",
].join("\n")
