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

						<p className="hint">Price source for the exotic token:</p>
						<div className="steps">
							{(
								[
									["curves", "Static bid/ask curves"],
									["uniswapV4", "Uniswap V4 positions"],
								] as const
							).map(([mode, label]) => (
								<button
									key={mode}
									type="button"
									className={`step ${state.fxPricing === mode ? "active" : ""}`}
									style={{ cursor: "pointer" }}
									onClick={() => setState((s) => ({ ...s, fxPricing: mode }))}
								>
									{label}
								</button>
							))}
						</div>

						{state.fxPricing === "uniswapV4" && (
							<div>
								<p className="hint">
									The pool acts as the price oracle and doubles as fill liquidity (withdrawn atomically when
									the wallet is short). Add at least one position; the optional price guard rejects fills when
									the live pool quote drifts from a reference price — set both guard fields or neither.
								</p>
								{state.fxPositions.map((position, index) => (
									// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
									<div className="row" key={index} style={{ marginBottom: "0.5rem" }}>
										<select
											value={position.chain}
											onChange={(e) =>
												setState((s) => ({
													...s,
													fxPositions: s.fxPositions.map((p, i) =>
														i === index ? { ...p, chain: e.target.value } : p,
													),
												}))
											}
										>
											{chains.map((c) => (
												<option key={c.meta.stateMachineId} value={c.meta.stateMachineId}>
													{c.meta.label}
												</option>
											))}
										</select>
										<input
											type="text"
											placeholder="position token id"
											style={{ maxWidth: "10rem" }}
											value={position.tokenId}
											onChange={(e) =>
												setState((s) => ({
													...s,
													fxPositions: s.fxPositions.map((p, i) =>
														i === index ? { ...p, tokenId: e.target.value } : p,
													),
												}))
											}
										/>
										<input
											type="text"
											placeholder="reference price (opt)"
											style={{ maxWidth: "10rem" }}
											value={position.referencePrice}
											onChange={(e) =>
												setState((s) => ({
													...s,
													fxPositions: s.fxPositions.map((p, i) =>
														i === index ? { ...p, referencePrice: e.target.value } : p,
													),
												}))
											}
										/>
										<input
											type="text"
											placeholder="max deviation bps (opt)"
											style={{ maxWidth: "10rem" }}
											value={position.maxDeviationBps}
											onChange={(e) =>
												setState((s) => ({
													...s,
													fxPositions: s.fxPositions.map((p, i) =>
														i === index ? { ...p, maxDeviationBps: e.target.value } : p,
													),
												}))
											}
										/>
										<button
											type="button"
											onClick={() =>
												setState((s) => ({ ...s, fxPositions: s.fxPositions.filter((_, i) => i !== index) }))
											}
										>
											✕
										</button>
									</div>
								))}
								<div className="row">
									<button
										type="button"
										onClick={() =>
											setState((s) => ({
												...s,
												fxPositions: [
													...s.fxPositions,
													{
														chain: chains[0]?.meta.stateMachineId ?? "",
														tokenId: "",
														referencePrice: "",
														maxDeviationBps: "",
													},
												],
											}))
										}
									>
										+ Add position
									</button>
									<label className="field" style={{ maxWidth: "16rem", margin: 0 }}>
										<span>Direction</span>
										<select
											value={state.fxSide}
											onChange={(e) =>
												setState((s) => ({ ...s, fxSide: e.target.value as "" | "ask" | "bid" }))
											}
										>
											<option value="">both directions</option>
											<option value="ask">ask only — sell exotic, accumulate stables</option>
											<option value="bid">bid only — buy exotic, accumulate exotic</option>
										</select>
									</label>
									<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
										<span>Spread (bps, optional)</span>
										<input
											type="text"
											value={state.fxSpreadBps}
											onChange={(e) => setState((s) => ({ ...s, fxSpreadBps: e.target.value }))}
										/>
									</label>
								</div>
							</div>
						)}

						{state.fxPricing === "curves" && (
							<p className="hint">
								Prices are exotic tokens per USD. Disabling one side is one-sided LP: bid-only keeps buying
								exotic (accumulates it), ask-only keeps selling it (accumulates stablecoins).
							</p>
						)}

						<div className="spread" style={state.fxPricing !== "curves" ? { display: "none" } : undefined}>
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
						{state.fxPricing === "curves" && state.fxBidEnabled && (
							<CurveEditor
								points={state.fxBid}
								onChange={(points) => setState((s) => ({ ...s, fxBid: points }))}
								amountLabel="Order size (USD)"
								valueLabel="Exotic per USD"
							/>
						)}

						<div
							className="spread"
							style={{ marginTop: "0.8rem", ...(state.fxPricing !== "curves" ? { display: "none" } : {}) }}
						>
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
						{state.fxPricing === "curves" && state.fxAskEnabled && (
							<CurveEditor
								points={state.fxAsk}
								onChange={(points) => setState((s) => ({ ...s, fxAsk: points }))}
								amountLabel="Order size (USD)"
								valueLabel="Exotic per USD"
							/>
						)}
						{state.fxPricing === "curves" && !state.fxBidEnabled && !state.fxAskEnabled && (
							<p className="error">Enable at least one direction.</p>
						)}
					</div>
				)}
			</div>
		</div>
	)
}
