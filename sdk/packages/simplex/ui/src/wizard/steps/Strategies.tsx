import { api } from "../../api"
import { CurveEditor } from "../../components/CurveEditor"
import { enabledChains } from "../state"
import type { StepProps } from "../Wizard"

export function StepStrategies({ state, setState }: StepProps) {
	const chains = enabledChains(state)

	const verifyToken = async (chainId: number) => {
		const chain = state.chains.find((c) => c.meta.chainId === chainId)
		if (!chain?.token1.trim() || !chain.rpcUrls[0]?.trim()) return
		try {
			const res = await api.post<{ ok: boolean; symbol?: string; decimals?: number; error?: string }>(
				"/api/setup/validate-token",
				{ rpcUrl: chain.rpcUrls[0].trim(), address: chain.token1.trim() },
			)
			setState((s) => ({
				...s,
				chains: s.chains.map((c) =>
					c.meta.chainId === chainId
						? {
								...c,
								tokenSymbol: res.ok ? `${res.symbol} (${res.decimals} decimals)` : undefined,
								tokenError: res.ok ? undefined : res.error,
							}
						: c,
				),
			}))
		} catch (err) {
			setState((s) => ({
				...s,
				chains: s.chains.map((c) =>
					c.meta.chainId === chainId
						? { ...c, tokenSymbol: undefined, tokenError: err instanceof Error ? err.message : String(err) }
						: c,
				),
			}))
		}
	}

	return (
		<div>
			<p className="hint">
				Strategies decide which orders are profitable to fill. Enable one or both; every order is evaluated by all
				enabled strategies and the most profitable fill wins.
			</p>

			<div className="card">
				<div className="spread">
					<h2>Stable — same-token transfers across chains (USDC→USDC, USDT→USDT)</h2>
					<label className="row">
						<input
							type="checkbox"
							checked={state.stableEnabled}
							onChange={(e) => setState((s) => ({ ...s, stableEnabled: e.target.checked }))}
						/>
						enabled
					</label>
				</div>
				{state.stableEnabled && (
					<div>
						<p className="hint">
							Your margin: the minimum basis points charged as a function of order size. Points are interpolated
							into a smooth curve — high bps keeps small orders worthwhile, low bps keeps large orders
							competitive. Needs at least 2 points.
						</p>
						<CurveEditor
							points={state.stableBps}
							onChange={(points) => setState((s) => ({ ...s, stableBps: points }))}
							amountLabel="Order size (USD)"
							valueLabel="Margin (bps)"
							minPoints={2}
						/>
					</div>
				)}
			</div>

			<div className="card">
				<div className="spread">
					<h2>HyperFX — stablecoin ↔ exotic token market making (e.g. cNGN)</h2>
					<label className="row">
						<input
							type="checkbox"
							checked={state.fxEnabled}
							onChange={(e) => setState((s) => ({ ...s, fxEnabled: e.target.checked }))}
						/>
						enabled
					</label>
				</div>
				{state.fxEnabled && (
					<div>
						<label className="field" style={{ maxWidth: "16rem" }}>
							<span>Max USD exposure per order (larger orders are partially filled)</span>
							<input
								type="text"
								value={state.fxMaxOrderUsd}
								onChange={(e) => setState((s) => ({ ...s, fxMaxOrderUsd: e.target.value }))}
							/>
						</label>

						<p className="hint">Exotic token contract per chain it exists on (at least one chain required):</p>
						{chains.map((chain) => (
							<label className="field" key={chain.meta.chainId}>
								<span>{chain.meta.label}</span>
								<div className="row">
									<input
										type="text"
										style={{ flex: 1 }}
										placeholder="0x… (leave empty if the token isn't on this chain)"
										value={chain.token1}
										onChange={(e) =>
											setState((s) => ({
												...s,
												chains: s.chains.map((c) =>
													c.meta.chainId === chain.meta.chainId
														? { ...c, token1: e.target.value, tokenSymbol: undefined, tokenError: undefined }
														: c,
												),
											}))
										}
									/>
									<button type="button" disabled={!chain.token1.trim()} onClick={() => verifyToken(chain.meta.chainId)}>
										Verify
									</button>
									{chain.tokenSymbol && <span className="badge ok">{chain.tokenSymbol}</span>}
									{chain.tokenError && <span className="badge err">{chain.tokenError}</span>}
								</div>
							</label>
						))}

						<p className="hint">
							Prices are exotic tokens per USD. Disabling one side is one-sided LP: bid-only keeps buying exotic
							(accumulates it), ask-only keeps selling it (accumulates stablecoins). Pool-based pricing from
							Uniswap V4 positions is configurable in the config file.
						</p>

						<div className="spread">
							<h2 style={{ fontSize: "0.95rem" }}>Bid curve — price when buying exotic from users</h2>
							<label className="row">
								<input
									type="checkbox"
									checked={state.fxBidEnabled}
									onChange={(e) => setState((s) => ({ ...s, fxBidEnabled: e.target.checked }))}
								/>
								enabled
							</label>
						</div>
						{state.fxBidEnabled && (
							<CurveEditor
								points={state.fxBid}
								onChange={(points) => setState((s) => ({ ...s, fxBid: points }))}
								amountLabel="Order size (USD)"
								valueLabel="Exotic per USD"
							/>
						)}

						<div className="spread" style={{ marginTop: "0.8rem" }}>
							<h2 style={{ fontSize: "0.95rem" }}>Ask curve — price when selling exotic to users</h2>
							<label className="row">
								<input
									type="checkbox"
									checked={state.fxAskEnabled}
									onChange={(e) => setState((s) => ({ ...s, fxAskEnabled: e.target.checked }))}
								/>
								enabled
							</label>
						</div>
						{state.fxAskEnabled && (
							<CurveEditor
								points={state.fxAsk}
								onChange={(points) => setState((s) => ({ ...s, fxAsk: points }))}
								amountLabel="Order size (USD)"
								valueLabel="Exotic per USD"
							/>
						)}
						{!state.fxBidEnabled && !state.fxAskEnabled && (
							<p className="error">Enable at least one direction.</p>
						)}
					</div>
				)}
			</div>
		</div>
	)
}
