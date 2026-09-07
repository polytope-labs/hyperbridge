import cngn from "../assets/tokens/cngn.png"
import dai from "../assets/tokens/dai.png"
import eurc from "../assets/tokens/eurc.png"
import unknown from "../assets/tokens/unknown.svg"
import usdc from "../assets/tokens/usdc.png"
import usdt from "../assets/tokens/usdt.png"
import zarp from "../assets/tokens/zarp.png"

const TOKEN_ICONS: Record<string, string> = {
	CNGN: cngn,
	DAI: dai,
	EURC: eurc,
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
