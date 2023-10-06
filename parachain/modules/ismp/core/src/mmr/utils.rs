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

use ismp_primitives::mmr::{LeafIndex, NodeIndex};

/// MMR nodes & size -related utilities.
pub struct NodesUtils {
    no_of_leaves: LeafIndex,
}

impl NodesUtils {
    /// Create new instance of MMR nodes utilities for given number of leaves.
    pub fn new(no_of_leaves: LeafIndex) -> Self {
        Self { no_of_leaves }
    }

    /// Calculate number of peaks in the MMR.
    pub fn number_of_peaks(&self) -> NodeIndex {
        self.number_of_leaves().count_ones() as NodeIndex
    }

    /// Return the number of leaves in the MMR.
    pub fn number_of_leaves(&self) -> LeafIndex {
        self.no_of_leaves
    }

    /// Calculate the total size of MMR (number of nodes).
    pub fn size(&self) -> NodeIndex {
        2 * self.no_of_leaves - self.number_of_peaks()
    }
}
