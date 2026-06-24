// Auto-generated, DO NOT EDIT
// Per-token ERC-20 storage slot overrides for tokens that don't follow the standard OZ layout
// (slot 0 = _balances, slot 1 = _allowances).
// To add or update entries, edit the "tokenSlots" field in the relevant chain entry
// in src/configs/config-mainnet.json (or config-testnet.json) and re-run codegen.
export const TOKEN_SLOT_OVERRIDES: Record<string, { balanceSlot: bigint; allowanceSlot: bigint }> = {
	// USDC (Circle FiatToken) — balances at slot 9, allowed at slot 10
	"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": { balanceSlot: 9n, allowanceSlot: 10n },
	// USDT (TetherToken) — balances at slot 2, allowed at slot 5
	"0xdac17f958d2ee523a2206206994597c13d831ec7": { balanceSlot: 2n, allowanceSlot: 5n },
	// USDC native Circle (FiatToken) — balances at slot 9, allowed at slot 10
	"0xaf88d065e77c8cc2239327c5edb3a432268e5831": { balanceSlot: 9n, allowanceSlot: 10n },
	// USDT (EIP-1967 proxy, impl layout) — balances at slot 51, allowed at slot 52
	"0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9": { balanceSlot: 51n, allowanceSlot: 52n },
	// USDC native Circle (FiatToken) — balances at slot 9, allowed at slot 10
	"0x0b2c639c533813f4aa9d7837caf62653d097ff85": { balanceSlot: 9n, allowanceSlot: 10n },
	// USDC native Circle (FiatToken) — balances at slot 9, allowed at slot 10
	"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": { balanceSlot: 9n, allowanceSlot: 10n },
	// cNGN (Nigerian Naira stablecoin, custom upgradeable layout) — balances at slot 201, allowed at slot 202
	"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": { balanceSlot: 201n, allowanceSlot: 202n },
	// BSC USDC (BEP-20 wrapper) — balances at slot 1, allowed at slot 2
	"0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d": { balanceSlot: 1n, allowanceSlot: 2n },
	// BSC USDT (BEP-20 TetherUSD) — balances at slot 1, allowed at slot 2
	"0x55d398326f99059ff775485246999027b3197955": { balanceSlot: 1n, allowanceSlot: 2n },
	// Gnosis USDC (FiatToken-based bridge) — balances at slot 9, allowed at slot 10
	"0x2a22f9c3b484c3629090feed35f17ff8f88f76f0": { balanceSlot: 9n, allowanceSlot: 10n },
	// USDC native Circle (FiatToken) — balances at slot 9, allowed at slot 10
	"0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": { balanceSlot: 9n, allowanceSlot: 10n }
}
