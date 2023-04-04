use crate::mmr::{LeafIndex, NodeIndex};
use alloc::vec::Vec;
use mmr_lib::helper;

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

    /// Calculate `LeafIndex` for the leaf that added `node_index` to the MMR.
    pub fn leaf_index_that_added_node(node_index: NodeIndex) -> LeafIndex {
        let rightmost_leaf_pos = Self::rightmost_leaf_node_index_from_pos(node_index);
        Self::leaf_node_index_to_leaf_index(rightmost_leaf_pos)
    }

    // Translate a _leaf_ `NodeIndex` to its `LeafIndex`.
    fn leaf_node_index_to_leaf_index(pos: NodeIndex) -> LeafIndex {
        if pos == 0 {
            return 0
        }
        let peaks = helper::get_peaks(pos);
        (pos + peaks.len() as u64) >> 1
    }

    // Starting from any node position get position of rightmost leaf; this is the leaf
    // responsible for the addition of node `pos`.
    fn rightmost_leaf_node_index_from_pos(pos: NodeIndex) -> NodeIndex {
        pos - (helper::pos_height_in_tree(pos) as u64)
    }

    /// Starting from any leaf index, get the sequence of positions of the nodes added
    /// to the mmr when this leaf was added (inclusive of the leaf's position itself).
    /// That is, all of these nodes are right children of their respective parents.
    pub fn _right_branch_ending_in_leaf(leaf_index: LeafIndex) -> Vec<NodeIndex> {
        let pos = helper::leaf_index_to_pos(leaf_index);
        let num_parents = leaf_index.trailing_ones() as u64;
        return (pos..=pos + num_parents).collect()
    }
}
