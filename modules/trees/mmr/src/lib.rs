use merkle_mountain_range::helper::{get_peaks, parent_offset, pos_height_in_tree, sibling_offset};

/// Converts a node's mmr position, to it's k-index. The k-index is the node's index within a layer
/// of the subtree.
pub fn mmr_position_to_k_index(mut leaves: Vec<u64>, mmr_size: u64) -> Vec<(u64, usize)> {
    let peaks = get_peaks(mmr_size);
    let mut leaves_with_k_indices = vec![];

    for peak in peaks {
        let leaves: Vec<_> = take_while_vec(&mut leaves, |pos| *pos <= peak);

        if leaves.len() > 0 {
            for pos in leaves {
                let height = pos_height_in_tree(peak);
                let mut index = 0;
                let mut parent_pos = peak;
                for height in (1..=height).rev() {
                    let left_child = parent_pos - parent_offset(height - 1);
                    let right_child = left_child + sibling_offset(height - 1);
                    index *= 2;
                    if left_child >= pos {
                        parent_pos = left_child;
                    } else {
                        parent_pos = right_child;
                        index += 1;
                    }
                }

                leaves_with_k_indices.push((pos, index));
            }
        }
    }

    leaves_with_k_indices
}

fn take_while_vec<T, P: Fn(&T) -> bool>(v: &mut Vec<T>, p: P) -> Vec<T> {
    for i in 0..v.len() {
        if !p(&v[i]) {
            return v.drain(..i).collect()
        }
    }
    v.drain(..).collect()
}
