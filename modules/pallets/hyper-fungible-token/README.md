# Pallet Hyper Fungible Token

Cross-chain token transfers between this substrate chain and one or more EVM
chains, paired with the `HyperFungibleToken` / `WrappedHyperFungibleToken`
Solidity contracts. Each registered token has a contract address per
destination chain; this pallet escrows or burns the local asset on `send` and
mints or releases the local asset on `on_accept` when a counterpart contract
sends back.

The pallet plugs into `pallet-ismp` as an [`IsmpModule`] under the well-known
module id `b"pall_hft"` — EVM contracts use this as the destination address
when targeting this pallet.

---

## Overview

A registered token is classified as one of two custody models:

- **Native** (`native = true`) — the asset originates on this chain. Outgoing
  transfers move the local balance into the pallet's escrow account; incoming
  messages release from escrow.
- **Non-native** (`native = false`) — the asset originates on a remote chain.
  Outgoing transfers burn the local representation; incoming messages mint
  fresh tokens.

The chain's own native currency (`T::NativeAssetId`) is always treated as
native, with `T::NativeCurrency` providing custody.

Decimals between this chain and each remote chain may differ; per-pair
`Precisions` storage records the EVM-side decimals so amounts get scaled at
the boundary.

---

## Storage

| Item | Type | Description |
|------|------|-------------|
| `TokenContracts` | `DoubleMap<StateMachine, AssetId → Vec<u8>>` | EVM contract address of a token on the given chain. Used as the `to` field on outgoing `DispatchPost`. |
| `ContractToAsset` | `DoubleMap<StateMachine, Vec<u8> → AssetId>` | Reverse lookup; on `on_accept` the source contract is mapped back to the local asset. |
| `NativeAssets` | `Map<AssetId → bool>` | Custody model flag (native vs non-native). |
| `Precisions` | `DoubleMap<AssetId, StateMachine → u8>` | EVM decimals for an `(asset, chain)` pair. |

---

## Extrinsics

| Call | Origin | Effect |
|------|--------|--------|
| `send(params)` | Signed | Lock or burn the local asset and dispatch a `Send` message to the paired contract on `params.destination`. Emits `TokenSent`. |
| `register_token(registration)` | `CreateOrigin` | Register a new asset, set its custody model (`native`) and per-chain contract+decimals config. Emits `TokenRegistered`. |
| `update_token(update)` | `CreateOrigin` | Add or remove chains from an existing token's configuration. |

---

## Events

| Event | Description |
|-------|-------------|
| `TokenSent { from, to, amount, dest, commitment }` | Outgoing transfer dispatched to `dest` via ISMP. |
| `TokenReceived { beneficiary, amount, source }` | Counterpart contract delivered an inbound transfer; tokens credited locally. |
| `TokenRefunded { beneficiary, amount, dest }` | Earlier outbound transfer to `dest` timed out; the escrowed/burned amount has been refunded locally. |
| `TokenRegistered { asset_id, native, chains }` | A new asset was registered for cross-chain transfer. |

---

## Errors

| Error | Cause |
|-------|-------|
| `UnregisteredAsset` | Asset has no row in `NativeAssets`/`TokenContracts`. |
| `TokenContractNotFound` | Asset is registered but no contract is configured for this destination chain. |
| `PalletAddressNotFound` | Counterpart EVM-side pallet address missing for this chain. |
| `DecimalsNotFound` | `Precisions` entry missing for the `(asset, chain)` pair. |
| `AssetTransferError` | The underlying `fungibles::Mutate` or `Currency::transfer` failed. |
| `DispatchError` | `pallet-ismp` rejected the outbound `DispatchPost`. |

---

## ISMP module behaviour

- `on_accept` — receives `Send` messages from the paired EVM contract. Maps
  the source contract back to a local asset via `ContractToAsset`, scales the
  amount using `Precisions`, then mints (non-native) or releases from escrow
  (native) to the beneficiary. Emits `TokenReceived`.
- `on_timeout` — refunds the original sender's balance from escrow or by
  re-minting. Emits `TokenRefunded`.
- `on_response` — unused; this pallet uses post-only messaging.

---

## Config trait

`Config` extends `frame_system::Config + pallet_ismp::Config`.

| Associated type | Constraint | Purpose |
|------|------------|---------|
| `Dispatcher` | `IsmpDispatcher<Account = AccountId, Balance = Self::Balance>` | Submits outgoing cross-chain requests via `pallet-ismp`. |
| `NativeCurrency` | `Currency<AccountId>` | Custodies the chain's own native asset for transfers where `asset_id == NativeAssetId`. |
| `CreateOrigin` | `EnsureOrigin<RuntimeOrigin>` | Origin allowed to call `register_token` and `update_token` (typically `EnsureRoot` or governance). |
| `Assets` | `fungibles::Mutate<AccountId> + fungibles::metadata::Inspect<AccountId>` | Fungible-asset backend for everything except the native currency. Burned on non-native sends, minted on inbound receives. |
| `NativeAssetId` | `Get<AssetId<Self>>` | The `AssetId` reserved for the chain's native currency. Sends with this id route through `NativeCurrency` instead of `Assets`. |
| `Decimals` | `Get<u8>` (`#[pallet::constant]`) | Decimals of the native currency, used for scaling at the boundary. |
| `EvmToSubstrate` | `EvmToSubstrate<Self>` | Authenticates incoming EVM-originated runtime calls by mapping an EVM address to a substrate account. The unit type `()` is acceptable if you do not need this dispatch path. |
| `WeightInfo` | `WeightInfo` | Benchmarked weights for `send` / `register_token` / `update_token`. `()` for prototypes. |

---

## Runtime integration

```rust
impl pallet_hyper_fungible_token::Config for Runtime {
    type Dispatcher = Ismp;
    type Assets = Assets;
    type NativeCurrency = Balances;
    type NativeAssetId = HftNativeAssetId;
    type CreateOrigin = EnsureRoot<AccountId>;
    type Decimals = HftDecimals;
    type EvmToSubstrate = ();
    type WeightInfo = ();
}
```

The runtime's [`IsmpRouter`] needs to dispatch inbound messages addressed to
the pallet's `PALLET_ID` (`b"pall_hft"`) to this pallet's `IsmpModule` impl:

```rust
impl IsmpRouter for Router {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
        match ModuleId::from_bytes(&id)? {
            pallet_hyper_fungible_token::PALLET_ID => Ok(Box::new(
                pallet_hyper_fungible_token::Pallet::<Runtime>::default(),
            )),
            // other modules
            _ => Err(anyhow::anyhow!("unknown module id")),
        }
    }
}
```
