import arbitrumLogo from "../assets/networks/arbitrum.svg"
import baseLogo from "../assets/networks/base.svg"
import bscLogo from "../assets/networks/bsc.svg"
import ethereumLogo from "../assets/networks/ethereum.svg"
import polygonLogo from "../assets/networks/polygon.svg"

interface ChainLogoProps {
	label: string
}

function chainFamily(label: string) {
	if (label.startsWith("Arbitrum")) return "arbitrum"
	if (label.startsWith("Base")) return "base"
	if (label.startsWith("Polygon")) return "polygon"
	if (label.startsWith("BNB") || label.startsWith("BSC")) return "bnb"
	if (label.startsWith("Ethereum")) return "ethereum"
}

const chainLogos = {
	arbitrum: arbitrumLogo,
	base: baseLogo,
	bnb: bscLogo,
	ethereum: ethereumLogo,
	polygon: polygonLogo,
}

/** Network variants share their parent chain's supplied, local logo asset. */
export function ChainLogo({ label }: ChainLogoProps) {
	const family = chainFamily(label)
	if (!family) {
		return (
			<span className="chain-logo chain-logo-fallback" aria-hidden="true">
				{label.slice(0, 2).toUpperCase()}
			</span>
		)
	}
	return <img className="chain-logo" src={chainLogos[family]} alt={`${label} logo`} />
}
