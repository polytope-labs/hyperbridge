// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use sp_core::H256;
use sp_mmr_primitives::{mmr_lib::helper, utils};
use sp_runtime::BuildStorage;

use crate::{mock::*, *};

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

fn new_leaf() {
    MMR::push(LeafData { a: 0, b: H256::random().0.to_vec() });
}

fn peaks_from_leaves_count(leaves_count: NodeIndex) -> Vec<NodeIndex> {
    let size = utils::NodesUtils::new(leaves_count).size();
    helper::get_peaks(size)
}

fn add_leaves(blocks: usize) {
    // given
    for _ in 0..blocks {
        new_leaf();
    }
    let _ = MMR::finalize();
}

#[test]
fn should_start_empty() {
    let _ = env_logger::try_init();
    new_test_ext().execute_with(|| {
        // given
        assert_eq!(
            crate::RootHash::<Test>::get(),
            "0000000000000000000000000000000000000000000000000000000000000000"
                .parse()
                .unwrap()
        );
        assert_eq!(crate::NumberOfLeaves::<Test>::get(), 0);
        assert_eq!(crate::Nodes::<Test>::get(0), None);

        // when
        add_leaves(1);

        // then
        assert_eq!(crate::NumberOfLeaves::<Test>::get(), 1);
        assert_eq!(crate::Nodes::<Test>::count(), 1);
    });
}

#[test]
fn should_construct_larger_mmr_correctly() {
    let _ = env_logger::try_init();
    new_test_ext().execute_with(|| {
        // when
        add_leaves(7);

        // then
        assert_eq!(crate::NumberOfLeaves::<Test>::get(), 7);
        let peaks = peaks_from_leaves_count(7);
        assert_eq!(peaks, vec![6, 9, 10]);
        for i in (0..=10).filter(|p| !peaks.contains(p)) {
            assert!(crate::Nodes::<Test>::get(i).is_none());
        }
    });
}

#[test]
fn should_calculate_the_size_correctly() {
    let _ = env_logger::try_init();

    let leaves = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 21];
    let sizes = vec![0, 1, 3, 4, 7, 8, 10, 11, 15, 16, 18, 19, 22, 23, 25, 26, 39];

    // size cross-check
    let mut actual_sizes = vec![];
    for s in &leaves[1..] {
        new_test_ext().execute_with(|| {
            let mut mmr = mmr::Mmr::<mmr::storage::RuntimeStorage, crate::mock::Test, _, _>::new(0);
            for _i in 0..*s {
                mmr.push(LeafData::default());
            }
            actual_sizes.push(mmr.size());
        })
    }
    assert_eq!(sizes[1..], actual_sizes[..]);
}
