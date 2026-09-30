import cngn from "../assets/tokens/cngn.png"
import dai from "../assets/tokens/dai.png"
import eurc from "../assets/tokens/eurc.png"
import bnb from "../assets/networks/bsc.svg"
import eth from "../assets/networks/ethereum.svg"
import matic from "../assets/networks/polygon.svg"
import unknown from "../assets/tokens/unknown.svg"
import usdc from "../assets/tokens/usdc.png"
import usdt from "../assets/tokens/usdt.png"
import zarp from "../assets/tokens/zarp.png"
import { ChainLogo } from "./ChainLogo"

const TOKEN_ICONS: Record<string, string> = {
	CNGN: cngn,
	DAI: dai,
	ETH: eth,
	EURC: eurc,
	BNB: bnb,
	MATIC: matic,
	USDC: usdc,
	USDT: usdt,
	ZARP: zarp,
}

export function TokenIcon({ symbol, size = "md" }: { symbol: string; size?: "sm" | "md" | "lg" }) {
	const normalized = symbol.trim().toUpperCase()
	return (
		<img
			className={`token-icon token-icon-${size}`}
			src={TOKEN_ICONS[normalized] ?? unknown}
			alt=""
			aria-hidden="true"
		/>
	)
}

export function TokenPairIcons({ tokenA, tokenB }: { tokenA: string; tokenB: string }) {
	return (
		<span className="token-pair-icons" aria-hidden="true">
			<TokenIcon symbol={tokenA} size="lg" />
			<TokenIcon symbol={tokenB} size="lg" />
		</span>
	)
}

/** A token as it sits on one chain: the token's icon, with the chain's logo as a badge on its corner. */
export function TokenOnChainIcon({ symbol, chain }: { symbol: string; chain: string }) {
	return (
		<span className="token-on-chain" aria-hidden="true">
			<TokenIcon symbol={symbol} size="md" />
			<span className="token-on-chain-badge">
				<ChainLogo label={chain} />
			</span>
		</span>
	)
}
