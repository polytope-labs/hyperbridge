use crate::{mock::*, *};
use std::{
    ops::Range,
    time::{SystemTime, UNIX_EPOCH},
};

use frame_support::traits::OnFinalize;
use ismp_primitives::mmr::MmrHasher;
use ismp_testsuite::{
    check_challenge_period, check_client_expiry, frozen_check, timeout_post_processing_check,
    write_outgoing_commitments,
};
use mmr_lib::MerkleProof;
use sp_core::{
    offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
    H256,
};

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
    let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
    ext.register_extension(OffchainDbExt::new(offchain.clone()));
    ext.register_extension(OffchainWorkerExt::new(offchain));
}

fn new_block() {
    let number = frame_system::Pallet::<Test>::block_number() + 1;
    let hash = H256::repeat_byte(number as u8);

    frame_system::Pallet::<Test>::reset_events();
    frame_system::Pallet::<Test>::initialize(&number, &hash, &Default::default());
    Ismp::on_finalize(number)
}

fn push_leaves(range: Range<u64>) -> Vec<NodeIndex> {
    // given
    let mut positions = vec![];
    for nonce in range {
        let post = ismp_rs::router::Post {
            source_chain: StateMachine::Kusama(2000),
            dest_chain: StateMachine::Kusama(2001),
            nonce,
            from: vec![0u8; 32],
            to: vec![1u8; 32],
            timeout_timestamp: 100 * nonce,
            data: vec![2u8; 64],
        };

        let request = Request::Post(post);
        let leaf = Leaf::Request(request);

        let pos = Pallet::<Test>::mmr_push(leaf.clone()).unwrap();
        positions.push(pos)
    }

    positions
}

#[test]
fn should_generate_proofs_correctly_for_single_leaf_mmr() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    let (root, positions) = ext.execute_with(|| {
        // push some leaves into the mmr
        let positions = push_leaves(0..1);
        new_block();
        let root = Pallet::<Test>::mmr_root();
        (root, positions)
    });
    ext.persist_offchain_overlay();

    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.
    register_offchain_ext(&mut ext);
    ext.execute_with(move || {
        let (leaves, proof) = Pallet::<Test>::generate_proof(vec![positions[0]]).unwrap();

        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof =
            MerkleProof::<DataOrHash<Test>, MmrHasher<Test, Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(vec![(positions[0], DataOrHash::Data(leaves[0].clone()))])
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>())
    })
}

#[test]
fn should_generate_and_verify_batch_proof_correctly() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    let (root, positions) = ext.execute_with(|| {
        // push some leaves into the mmr
        let positions = push_leaves(0..12);
        new_block();
        let root = Pallet::<Test>::mmr_root();
        (root, positions)
    });
    ext.persist_offchain_overlay();

    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.
    register_offchain_ext(&mut ext);
    ext.execute_with(move || {
        let indices = vec![positions[0], positions[3], positions[2], positions[5]];
        let (leaves, proof) = Pallet::<Test>::generate_proof(indices.clone()).unwrap();

        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof =
            MerkleProof::<DataOrHash<Test>, MmrHasher<Test, Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(
                indices
                    .into_iter()
                    .zip(leaves.into_iter().map(|leaf| DataOrHash::Data(leaf)))
                    .collect(),
            )
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>())
    })
}

#[test]
fn should_generate_and_verify_batch_proof_for_leaves_inserted_across_multiple_blocks_correctly() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    let (root, positions) = ext.execute_with(|| {
        // push some leaves into the mmr
        let mut positions = push_leaves(0..6);
        new_block();
        let positions_second = push_leaves(6..12);
        new_block();
        let root = Pallet::<Test>::mmr_root();
        positions.extend_from_slice(&positions_second);
        (root, positions)
    });
    ext.persist_offchain_overlay();

    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.
    register_offchain_ext(&mut ext);
    ext.execute_with(move || {
        let indices = vec![positions[0], positions[9], positions[2], positions[8]];
        let (leaves, proof) = Pallet::<Test>::generate_proof(indices.clone()).unwrap();

        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof =
            MerkleProof::<DataOrHash<Test>, MmrHasher<Test, Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(
                indices
                    .into_iter()
                    .zip(leaves.into_iter().map(|leaf| DataOrHash::Data(leaf)))
                    .collect(),
            )
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>())
    })
}

fn set_timestamp() {
    Timestamp::set_timestamp(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
    );
}

#[test]
fn check_for_duplicate_requests_and_responses() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp();
        let host = Host::<Test>::default();
        write_outgoing_commitments(&host).unwrap();
    })
}

#[test]
fn should_reject_updates_within_challenge_period() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp();
        let host = Host::<Test>::default();
        check_challenge_period(&host).unwrap()
    })
}

#[test]
fn should_reject_messages_for_frozen_state_machines() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp();
        let host = Host::<Test>::default();
        frozen_check(&host).unwrap()
    })
}

#[test]
fn should_reject_expired_check_clients() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp();
        let host = Host::<Test>::default();
        check_client_expiry(&host).unwrap()
    })
}
#[test]
fn should_process_timeouts_correctly() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp();
        let host = Host::<Test>::default();
        timeout_post_processing_check(&host).unwrap()
    })
}
