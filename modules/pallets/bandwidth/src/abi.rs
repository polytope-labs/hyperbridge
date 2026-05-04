// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! ABI codec for the purchase message dispatched by `BandwidthMarket.sol`.
//! Field layout must stay in lockstep with the Solidity struct.

use alloc::{format, vec::Vec};
use alloy_sol_macro::sol;
use alloy_sol_types::SolType;
use primitive_types::H160;

sol! {
    #![sol(all_derives)]

    struct BandwidthPurchaseMsgAbi {
        address app;
        uint256 bytesPurchased;
        uint256 amountPaid;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseMessage {
    pub app: H160,
    pub bytes_purchased: u128,
    pub amount_paid_18d: u128,
}

/// Both numeric fields are `uint256` on the wire but bounded by the
/// stablecoin supply in practice; rejects values that don't fit `u128`.
pub fn decode_purchase_msg(body: &[u8]) -> Result<PurchaseMessage, anyhow::Error> {
    let abi = BandwidthPurchaseMsgAbi::abi_decode(body)
        .map_err(|err| anyhow::anyhow!(format!("invalid bandwidth purchase ABI: {err:?}")))?;

    let bytes_purchased: u128 = abi
        .bytesPurchased
        .try_into()
        .map_err(|_| anyhow::anyhow!("bytesPurchased exceeds u128"))?;
    let amount_paid_18d: u128 = abi
        .amountPaid
        .try_into()
        .map_err(|_| anyhow::anyhow!("amountPaid exceeds u128"))?;

    Ok(PurchaseMessage {
        app: H160(abi.app.into()),
        bytes_purchased,
        amount_paid_18d,
    })
}

/// Inverse of [`decode_purchase_msg`]; used by tests and a future
/// substrate-source purchase path.
pub fn encode_purchase_msg(msg: &PurchaseMessage) -> Vec<u8> {
    let abi = BandwidthPurchaseMsgAbi {
        app: alloy_primitives::Address::from(msg.app.0),
        bytesPurchased: alloy_primitives::U256::from(msg.bytes_purchased),
        amountPaid: alloy_primitives::U256::from(msg.amount_paid_18d),
    };
    BandwidthPurchaseMsgAbi::abi_encode(&abi)
}
