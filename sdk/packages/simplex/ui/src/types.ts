// Wire contracts and config types come straight from src/ — type-only
// imports, erased at build, so the server and the browser cannot drift.

export type { InitChainMeta as ChainDefault, InitNetwork as Network } from "@/cli/init/chains"
// The wizard writes the binary's config file, signer block included — that is
// `FillerConfigFile`, not the library's `FillerTomlConfig`.
export type { FillerConfigFile as FillerConfig } from "@/config/filler-toml"
export type { CurvePoint, PriceCurvePoint as PricePoint } from "@/config/interpolated-curve"
export type { PairConfig } from "@/config/pairs"
export type {
	ActivityEventDto,
	AdminStrategyDto,
	BalanceSnapshot,
	BidDto,
	BidStatsDto,
	ChainRowDto,
	ChainsDto,
	ConfigDto,
	KnownToken,
	KnownVault,
	SendTokenOption,
	SetupDefaults,
	Status,
	StatusInit,
	StatusOperator,
	VaultSweepDto,
	WalletTxDto,
} from "@/services/server/dto"
