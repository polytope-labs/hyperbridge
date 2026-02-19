## Rebalancing Rulebook

This document defines how rebalancing should work for USDC and USDT balances across chains.

### 1. Configuration

- **Trigger percentage**

    - A single global `triggerPercentage` \(p\), between 0 and 1.
    - Example: `0.5` means trigger when a balance falls to 50% of its base or lower.

- **Per-chain, per-asset base balances**
    - For each chain `c` and each asset `USDC` and `USDT`:
        - `baseBalance[c]["USDC"]` – target USDC balance on that chain.
        - `baseBalance[c]["USDT"]` – target USDT balance on that chain.
    - USDC and USDT are both treated as \$1:
        - USD value = normalized token amount using token decimals  
          (same logic as `ContractInteractionService.getTokenUsdValue`).

### 2. Trigger Conditions

- For each chain `c` and asset `a ∈ {USDC, USDT}`:
    - Let:
        - `B[c,a]` = base balance for asset `a` on chain `c`.
        - `C[c,a]` = current balance for asset `a` on chain `c`.
        - `T[c,a] = B[c,a] * (1 - p)` = trigger threshold.
    - **Trigger rule:**
        - If `C[c,a] <= T[c,a]` for any asset on chain `c`, we trigger rebalancing:
            - Only for **that chain `c`**, and
            - Only for the **asset(s)** that breached the threshold.

### 3. Surplus and Deficit

- For any chain `c` and asset `a`:
    - **Deficit (needs funds)**:
        - If `C[c,a] < B[c,a]`, deficit is `D[c,a] = B[c,a] - C[c,a] > 0`.
    - **Surplus (can supply funds)**:
        - If `C[c,a] > B[c,a]`, surplus is `S[c,a] = C[c,a] - B[c,a] > 0`.
    - When using a chain as a source, we must not bring it below its base:
        - After transfers, `C'[c,a] >= B[c,a]` must hold for all sources.

### 4. Cross-Chain Rebalancing

When chain `c` is triggered and has a deficit for asset `a`:

1. Compute deficit:

    - `D[c,a]` = deficit for asset `a` on chain `c` = `B[c,a] - C[c,a]`.

2. Compute global surpluses for that asset on other chains:

    - For every chain `s != c`, compute `S[s,a]` (surplus for asset `a`).
    - Only chains with `S[s,a] > 0` can be sources.

3. **Total surplus check:**

    - Let `TotalSurplus[a] = sum over s != c of S[s,a]`.
    - If `TotalSurplus[a] < D[c,a]`:
        - **Throw an error**:
            - "Insufficient surplus to bring chain `c` asset `a` back to base."
        - Do not perform partial rebalancing for that trigger.

4. Build lists:

    - **Deficits**: currently only the triggered chain `c` for asset `a`
      (design supports more deficit chains in the future).
    - **Surpluses**: all `(sourceChain, surplusAmount)` pairs where surplus is `> 0`.

5. Use these to plan cross-chain transfers for asset `a`.

**Important:** We only transfer the same asset type:

- USDC deficits can only be filled by USDC transfers from other chains.
- USDT deficits can only be filled by USDT transfers from other chains.
- No swapping between internal USDC and USDT to avoid slippage.

### 5. Algorithm to Minimize Cross-Chain Transfers

Goal: satisfy all external deficits using the **fewest number of transfers**.

- Inputs:

    - Deficit chains `i` with deficits `D[i] > 0`.
    - Surplus chains `j` with surpluses `S[j] > 0`.
    - Each transfer is a continuous amount from one surplus chain to one deficit chain.
    - No source may go below its base.

- **Greedy surplus–deficit matching algorithm:**

    1. Sort deficit chains in **descending order** of `D[i]` (largest deficits first).
    2. Sort surplus chains in **descending order** of `S[j]` (largest surpluses first).
    3. For each deficit chain `i` in order:
        - While `D[i] > 0`:
            - Take the current largest remaining surplus `S[j]`.
            - Transfer `x = min(D[i], S[j])` from chain `j` to chain `i`.
            - Update:
                - `D[i] = D[i] - x`.
                - `S[j] = S[j] - x`.
            - Record a transfer `(source = j, dest = i, asset, amount = x)`.
            - If `S[j] == 0`, move to the next surplus chain.
        - Stop when `D[i] == 0`.

- Properties:
    - Every transfer either:
        - Fully resolves a deficit, or
        - Fully consumes a surplus, or both.
    - This avoids spreading deficits and surpluses over more pairs than needed.
    - This is **not** a shortest-path algorithm; it is a
      **greedy surplus–deficit matching** strategy that aims to minimize
      the number of non-zero transfers.

### 6. Executing Transfers

- For each planned cross-chain transfer `(sourceChain, destChain, asset, amount)`:

    - Build a `RebalanceOptions` / `CexRebalanceOptions`-style object:
        - `amount`: human-readable string (e.g. `"5000"`).
        - `coin`: `"USDC"` or `"USDT"`.
        - `source`: state machine ID of the source chain (e.g. `"EVM-42161"`).
        - `destination`: state machine ID of the destination chain.
    - Call `RebalancingService.rebalance`:
        - It chooses the route:
            - `CCTP` for USDC where supported on both chains.
            - `USDT0` for USDT where supported on both chains.
            - Binance CEX as a fallback when needed.

- Transfers for a given rebalance run may be executed in parallel
  (e.g. with `Promise.all`), because they are independent.

### 7. Error Handling and Edge Cases

- **Insufficient global surplus for an asset**:

    - If total surplus across all other chains for the asset is less than the
      deficit for the triggered chain:
        - Throw an error and do not perform a partial rebalance.

- **Route unavailable for a specific source/destination pair**:

    - If `RebalancingService.rebalance` (or its routing logic) cannot find
      a route for a planned transfer:
        - Surface this as an error for that rebalance attempt.

- **Precision and decimals**:
    - All comparisons and planning use normalized amounts (token units scaled
      by decimals).
    - On execution, convert planned amounts back to on-chain units using
      the token decimals (same approach as in `ContractInteractionService`).
