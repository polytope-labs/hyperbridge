// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Subxt [`Config`] implementations

use codec::Encode;
use sp_core::{blake2_256, keccak_256};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
};

/// Implements [`subxt::Config`] for parachains with keccak as their hashing algorithm
#[derive(Clone)]
pub struct KeccakParachain;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        keccak_256(s).into()
    }
}

impl subxt::Config for KeccakParachain {
    type Index = u32;
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = KeccakHasher;
    type Header = SubstrateHeader<u32, KeccakHasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

/// Implements [`subxt::Config`] for parachains with blake2 as their hashing algorithm
#[derive(Clone)]
pub struct Blake2Parachain;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct Blake2Hasher;

impl Hasher for Blake2Hasher {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        blake2_256(s).into()
    }
}

impl subxt::Config for Blake2Parachain {
    type Index = u32;
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = Blake2Hasher;
    type Header = SubstrateHeader<u32, Blake2Hasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}
