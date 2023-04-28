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
