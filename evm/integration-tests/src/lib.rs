#![allow(unused_parens)]

#[cfg(test)]
mod tests;

pub use ethers::{abi::Token, types::U256, utils::keccak256};
use merkle_mountain_range::{util::MemMMR, Error, Merge};
use pallet_ismp::mmr_primitives::{DataOrHash, MmrHasher};
use primitive_types::H256;
use rs_merkle::Hasher;

#[derive(Clone, Default)]
pub struct Keccak256;

impl Hasher for Keccak256 {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        keccak256(data)
    }
}

impl ismp::util::Keccak256 for Keccak256 {
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        keccak256(bytes).into()
    }
}

struct MergeKeccak;

impl Merge for MergeKeccak {
    type Item = NumberHash;
    fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item, Error> {
        let mut concat = vec![];
        concat.extend(&lhs.0);
        concat.extend(&rhs.0);
        let hash = keccak256(&concat);
        Ok(NumberHash(hash.to_vec().into()))
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Default)]
struct NumberHash(pub Vec<u8>);

impl From<u32> for NumberHash {
    fn from(num: u32) -> Self {
        let hash = keccak256(&num.to_le_bytes());
        NumberHash(hash.to_vec())
    }
}

pub type Mmr = MemMMR<DataOrHash, MmrHasher<Keccak256>>;

#[cfg(test)]
mod mmr_tests {
    use super::*;
    use crate::Mmr;
    use hex_literal::hex;
    use merkle_mountain_range::{leaf_index_to_mmr_size, leaf_index_to_pos, MerkleProof};
    use pallet_ismp::mmr_primitives::DataOrHash;
    use primitive_types::H256;

    #[test]
    fn test_mmr_proof() {
        let size = leaf_index_to_mmr_size(1616);
        let proof = vec![
            DataOrHash::Hash(H256(hex!(
                "b5ba4d6e5ea759a8aa2ef3480d836a61253d6d0c6e886e0ede3b332c63b4e160"
            ))),
            DataOrHash::Hash(H256(hex!(
                "eeca66a0cba259c50eb3baa38d205af30d7cb83120feb76a4300d0d37b17a1f8"
            ))),
            DataOrHash::Hash(H256(hex!(
                "b126f476a09ec30789dbb04f9c5495efc423fdb2b32528d83c2b6917915bd486"
            ))),
            DataOrHash::Hash(H256(hex!(
                "fdf2fe07c35b9666d40ead2679e9e08b4cef69a06a8a6847e1886ed70cf48edb"
            ))),
            DataOrHash::Hash(H256(hex!(
                "25730d6650894ac1785b6c1b38e4c5b507fc9386a0a792e015e1fd99740828af"
            ))),
            DataOrHash::Hash(H256(hex!(
                "3e6b8c07345e2c358afd103457256c769ce6c04e16aa5c1bc4d4d2795a29e49c"
            ))),
            DataOrHash::Hash(H256(hex!(
                "3363b5e293ed1d28c0640bea57ccd6446953525620141e600f37bb10e494426f"
            ))),
            DataOrHash::Hash(H256(hex!(
                "3a421ae4caea6a9adc04ee8d54940e1ea0fee33cea3aa0ac263170ea6758f99b"
            ))),
            DataOrHash::Hash(H256(hex!(
                "00d74749f60056f148369a2da31fcc81aedcae9770b985c303e553033f9859a6"
            ))),
        ];
        let proof = MerkleProof::<_, MmrHasher<Keccak256>>::new(size, proof);

        let leaves = vec![
            (
                leaf_index_to_pos(1536),
                DataOrHash::Hash(H256(hex!(
                    "81f85f9cf38ba86d8c2024d69a0f2b0fa1f314a8abc09a45c7dbe41f0ea3b110"
                ))),
            ),
            (
                leaf_index_to_pos(1537),
                DataOrHash::Hash(H256(hex!(
                    "14e6c3fd40f465dfb541c267181a438e0467a95c8dee66de770133fea4462ec5"
                ))),
            ),
            (
                leaf_index_to_pos(1538),
                DataOrHash::Hash(H256(hex!(
                    "1858e0a44b8e0a2dec6198c05102f94aa83b4f6f9e36aef0f2c1d79a65d89a0c"
                ))),
            ),
            (
                leaf_index_to_pos(1539),
                DataOrHash::Hash(H256(hex!(
                    "82a6e37a8092ca7c34af62f89ec58725984ef0d49e07919aacd3d2b82b97d8a7"
                ))),
            ),
            (
                leaf_index_to_pos(1540),
                DataOrHash::Hash(H256(hex!(
                    "ca627b3f74ca6277e1e67fbb87f07902995f7e4fae2962f81b39dc3a415e73c1"
                ))),
            ),
            (
                leaf_index_to_pos(1541),
                DataOrHash::Hash(H256(hex!(
                    "81db80fad0058f94e2fd10a8868d4a5f0f705f8bfa4f6d5e3f0f099552907cf4"
                ))),
            ),
            (
                leaf_index_to_pos(1542),
                DataOrHash::Hash(H256(hex!(
                    "b65d9932cb2b113fa3a06bd63ea3587d05fbcad77777e1e61b8a00c4a0eebcd1"
                ))),
            ),
            (
                leaf_index_to_pos(1543),
                DataOrHash::Hash(H256(hex!(
                    "3fbff52a995e9d3b9f6a8b6fc3cea789466a6066face509088245707f6c342ab"
                ))),
            ),
            (
                leaf_index_to_pos(1544),
                DataOrHash::Hash(H256(hex!(
                    "1a73f5e434d821543b850fcb28ec068d28f615603626769576a68edaa2815949"
                ))),
            ),
            (
                leaf_index_to_pos(1545),
                DataOrHash::Hash(H256(hex!(
                    "258bedbe1991acea86079ce1a46b3356a87910c06da25b5343e5c0cb6a6094ce"
                ))),
            ),
            (
                leaf_index_to_pos(1590),
                DataOrHash::Hash(H256(hex!(
                    "0f7a720cb160bf6cddcf322549ad3509ef15ecdc1275e3d73b078253b7b379bd"
                ))),
            ),
            (
                leaf_index_to_pos(1591),
                DataOrHash::Hash(H256(hex!(
                    "326872ebb588c014e2decc2dc12ead55e6870e100436060c8ab4777e5277af6e"
                ))),
            ),
            (
                leaf_index_to_pos(1592),
                DataOrHash::Hash(H256(hex!(
                    "1eec48569e4a29b683b8f3acf31b786f36c24da04fee9570085bedb8c3ce91ad"
                ))),
            ),
            (
                leaf_index_to_pos(1593),
                DataOrHash::Hash(H256(hex!(
                    "aaba34ae45f603bc9bb7b55ff983617c9c46ffc14763289ee6a5615a97165950"
                ))),
            ),
            (
                leaf_index_to_pos(1594),
                DataOrHash::Hash(H256(hex!(
                    "e9db33d9c81ebe7b1b6511b110a2c038e2e9f37b0e83affb1769b55aa14f69bb"
                ))),
            ),
            (
                leaf_index_to_pos(1595),
                DataOrHash::Hash(H256(hex!(
                    "b89f84b3089ac90e14b6e32d737b666bfb2584f2738d480a9734b9e3411c97d1"
                ))),
            ),
            (
                leaf_index_to_pos(1596),
                DataOrHash::Hash(H256(hex!(
                    "0982dd72d7f46598ecadaf528d82de610bb2e05d5a7498d55e35e53a3eaa6895"
                ))),
            ),
            (
                leaf_index_to_pos(1597),
                DataOrHash::Hash(H256(hex!(
                    "f54a14389dea60efba42c4ceb42305508d4fb5ef4ff25dd38c247c9e46ad19a3"
                ))),
            ),
            (
                leaf_index_to_pos(1598),
                DataOrHash::Hash(H256(hex!(
                    "719cc6267699d64e7586fa8ff05d44d3fbfc3413ec2346d4aacb1f0f74b5606e"
                ))),
            ),
            (
                leaf_index_to_pos(1599),
                DataOrHash::Hash(H256(hex!(
                    "b528dfaee3c426cfd77cc52a9e6bf763907930f64a8710b5a3540f8ae0c4519d"
                ))),
            ),
        ];

        dbg!(proof.calculate_root(leaves));
    }
}
