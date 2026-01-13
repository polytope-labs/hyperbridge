// Copyright (C) Polytope Labs Ltd.
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

//! BLS12-381 cryptographic type definitions.

use crate::ssz::ByteVector;

/// Length of a BLS12-381 public key in bytes (compressed G1 point).
pub const BLS_PUBLIC_KEY_BYTES_LEN: usize = 48;

/// Length of a BLS12-381 signature in bytes (compressed G2 point).
pub const BLS_SIGNATURE_BYTES_LEN: usize = 96;

/// A BLS12-381 public key (48 bytes compressed).
pub type BlsPublicKey = ByteVector<BLS_PUBLIC_KEY_BYTES_LEN>;

/// A BLS12-381 signature (96 bytes compressed).
pub type BlsSignature = ByteVector<BLS_SIGNATURE_BYTES_LEN>;
