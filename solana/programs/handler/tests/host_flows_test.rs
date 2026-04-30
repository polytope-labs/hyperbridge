//! Native flow tests for `handle_consensus` and `handle_post_requests`
//! against a recording `IsmpHost`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId,
        StateCommitment as IsmpStateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error as IsmpError,
    host::{IsmpHost, StateMachine},
    messaging::{hash_request, ConsensusMessage, Keccak256, Message, Proof, RequestMessage},
    router::{IsmpRouter, PostRequest, PostResponse, Request, Response},
};
use parity_scale_codec::{Decode, Encode};
use primitive_types::H256;
use sha3::{Digest, Keccak256 as Sha3Keccak};
use sp1_beefy_verifier::{
    fixtures::{sp1_vkey_hash, trusted_state_bytes, wire_proof_bytes},
    ConsensusState as BeefyConsensusState,
};

use handler::ismp::{Sp1BeefyConsensusClient, SolanaCpiModule};
use polkadot_sdk::{
    sp_core::Blake2Hasher,
    sp_trie::{
        empty_trie_root, recorder::Recorder, LayoutV0, MemoryDB, Trie, TrieDBBuilder,
        TrieDBMutBuilder, TrieMut,
    },
};

const BEFY: ConsensusClientId = *b"BEFY";
const SOLANA_STATE_MACHINE: StateMachine = StateMachine::Substrate(*b"sola");

struct RecordingHost {
    consensus_state: RefCell<Option<Vec<u8>>>,
    consensus_last_updated_secs: RefCell<i64>,
    state_commitments:
        RefCell<BTreeMap<(StateMachineId, u64), (IsmpStateCommitment, Duration)>>,
    latest_heights: RefCell<BTreeMap<StateMachineId, u64>>,
    now_unix_secs: i64,
    frozen: bool,
    unbonding_period_secs: u64,
    sp1_vkey_hash: [u8; 32],
    commit_header_index: Option<usize>,

    stored_consensus_states: RefCell<Vec<Vec<u8>>>,
    stored_consensus_update_times: RefCell<Vec<Duration>>,
    stored_state_commitments: RefCell<Vec<(StateMachineHeight, IsmpStateCommitment)>>,
    stored_state_machine_update_times: RefCell<Vec<(StateMachineHeight, Duration)>>,
    stored_latest_heights: RefCell<Vec<StateMachineHeight>>,
    stored_request_receipts: RefCell<Vec<H256>>,
    deleted_request_receipts: RefCell<Vec<H256>>,
}

impl RecordingHost {
    fn new(trusted_state: Vec<u8>) -> Self {
        Self {
            consensus_state: RefCell::new(Some(trusted_state)),
            consensus_last_updated_secs: RefCell::new(1_000_000),
            state_commitments: RefCell::new(BTreeMap::new()),
            latest_heights: RefCell::new(BTreeMap::new()),
            now_unix_secs: 1_500_000,
            frozen: false,
            unbonding_period_secs: 60 * 60 * 24 * 365 * 100,
            sp1_vkey_hash: sp1_vkey_hash(),
            commit_header_index: Some(0),
            stored_consensus_states: RefCell::new(Vec::new()),
            stored_consensus_update_times: RefCell::new(Vec::new()),
            stored_state_commitments: RefCell::new(Vec::new()),
            stored_state_machine_update_times: RefCell::new(Vec::new()),
            stored_latest_heights: RefCell::new(Vec::new()),
            stored_request_receipts: RefCell::new(Vec::new()),
            deleted_request_receipts: RefCell::new(Vec::new()),
        }
    }
}

impl Keccak256 for RecordingHost {
    fn keccak256(bytes: &[u8]) -> H256 {
        let mut h = Sha3Keccak::new();
        h.update(bytes);
        H256::from_slice(h.finalize().as_slice())
    }
}

fn outbound<T>(m: &'static str) -> Result<T, IsmpError> {
    Err(IsmpError::Custom(format!("{m}: outbound unsupported in test")))
}

impl IsmpHost for RecordingHost {
    fn host_state_machine(&self) -> StateMachine {
        SOLANA_STATE_MACHINE
    }

    fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, IsmpError> {
        Ok(self.latest_heights.borrow().get(&id).copied().unwrap_or(0))
    }

    fn state_machine_commitment(
        &self,
        h: StateMachineHeight,
    ) -> Result<IsmpStateCommitment, IsmpError> {
        self.state_commitments
            .borrow()
            .get(&(h.id, h.height))
            .map(|(c, _)| c.clone())
            .ok_or(IsmpError::StateCommitmentNotFound { height: h })
    }

    fn consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Duration, IsmpError> {
        let ts = *self.consensus_last_updated_secs.borrow();
        if ts < 0 {
            return Err(IsmpError::ConsensusStateNotFound { consensus_state_id });
        }
        Ok(Duration::from_secs(ts as u64))
    }

    fn state_machine_update_time(
        &self,
        h: StateMachineHeight,
    ) -> Result<Duration, IsmpError> {
        self.state_commitments
            .borrow()
            .get(&(h.id, h.height))
            .map(|(_, t)| *t)
            .ok_or(IsmpError::StateCommitmentNotFound { height: h })
    }

    fn consensus_client_id(
        &self,
        _id: ConsensusStateId,
    ) -> Option<ConsensusClientId> {
        Some(BEFY)
    }

    fn consensus_state(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Vec<u8>, IsmpError> {
        self.consensus_state
            .borrow()
            .clone()
            .ok_or(IsmpError::ConsensusStateNotFound { consensus_state_id })
    }

    fn timestamp(&self) -> Duration {
        Duration::from_secs(self.now_unix_secs.max(0) as u64)
    }

    fn is_consensus_client_frozen(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<(), IsmpError> {
        if self.frozen {
            Err(IsmpError::FrozenConsensusClient { consensus_state_id })
        } else {
            Ok(())
        }
    }

    fn request_commitment(&self, _r: H256) -> Result<(), IsmpError> {
        outbound("request_commitment")
    }
    fn response_commitment(&self, _r: H256) -> Result<(), IsmpError> {
        outbound("response_commitment")
    }
    fn next_nonce(&self) -> u64 {
        0
    }
    fn request_receipt(&self, _r: &Request) -> Option<()> {
        None
    }
    fn response_receipt(&self, _r: &Response) -> Option<()> {
        None
    }

    fn store_consensus_state_id(
        &self,
        _id: ConsensusStateId,
        _client_id: ConsensusClientId,
    ) -> Result<(), IsmpError> {
        outbound("store_consensus_state_id")
    }

    fn store_consensus_state(
        &self,
        _id: ConsensusStateId,
        consensus_state: Vec<u8>,
    ) -> Result<(), IsmpError> {
        *self.consensus_state.borrow_mut() = Some(consensus_state.clone());
        self.stored_consensus_states.borrow_mut().push(consensus_state);
        Ok(())
    }

    fn store_unbonding_period(
        &self,
        _id: ConsensusStateId,
        _period: u64,
    ) -> Result<(), IsmpError> {
        outbound("store_unbonding_period")
    }

    fn store_consensus_update_time(
        &self,
        _id: ConsensusStateId,
        timestamp: Duration,
    ) -> Result<(), IsmpError> {
        *self.consensus_last_updated_secs.borrow_mut() = timestamp.as_secs() as i64;
        self.stored_consensus_update_times.borrow_mut().push(timestamp);
        Ok(())
    }

    fn store_state_machine_update_time(
        &self,
        h: StateMachineHeight,
        timestamp: Duration,
    ) -> Result<(), IsmpError> {
        self.stored_state_machine_update_times
            .borrow_mut()
            .push((h, timestamp));
        if let Some(entry) = self.state_commitments.borrow_mut().get_mut(&(h.id, h.height)) {
            entry.1 = timestamp;
        }
        Ok(())
    }

    fn store_state_machine_commitment(
        &self,
        h: StateMachineHeight,
        commitment: IsmpStateCommitment,
    ) -> Result<(), IsmpError> {
        self.state_commitments
            .borrow_mut()
            .insert((h.id, h.height), (commitment.clone(), Duration::from_secs(0)));
        self.stored_state_commitments
            .borrow_mut()
            .push((h, commitment));
        Ok(())
    }

    fn delete_state_commitment(
        &self,
        _h: StateMachineHeight,
    ) -> Result<(), IsmpError> {
        outbound("delete_state_commitment")
    }

    fn freeze_consensus_client(
        &self,
        _id: ConsensusStateId,
    ) -> Result<(), IsmpError> {
        outbound("freeze_consensus_client")
    }

    fn store_latest_commitment_height(
        &self,
        h: StateMachineHeight,
    ) -> Result<(), IsmpError> {
        self.latest_heights.borrow_mut().insert(h.id, h.height);
        self.stored_latest_heights.borrow_mut().push(h);
        Ok(())
    }

    fn delete_request_commitment(&self, _r: &Request) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_request_commitment")
    }
    fn delete_response_commitment(&self, _r: &PostResponse) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_response_commitment")
    }
    fn delete_request_receipt(&self, r: &Request) -> Result<Vec<u8>, IsmpError> {
        let commitment = hash_request::<Self>(r);
        self.deleted_request_receipts.borrow_mut().push(commitment);
        Ok(Vec::new())
    }
    fn delete_response_receipt(&self, _r: &Response) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_response_receipt")
    }
    fn store_request_receipt(
        &self,
        r: &Request,
        signer: &Vec<u8>,
    ) -> Result<Vec<u8>, IsmpError> {
        let commitment = hash_request::<Self>(r);
        self.stored_request_receipts.borrow_mut().push(commitment);
        Ok(signer.clone())
    }
    fn store_response_receipt(
        &self,
        _r: &Response,
        _s: &Vec<u8>,
    ) -> Result<Vec<u8>, IsmpError> {
        outbound("store_response_receipt")
    }
    fn store_request_commitment(
        &self,
        _r: &Request,
        _m: Vec<u8>,
    ) -> Result<(), IsmpError> {
        outbound("store_request_commitment")
    }
    fn store_response_commitment(
        &self,
        _r: &PostResponse,
        _m: Vec<u8>,
    ) -> Result<(), IsmpError> {
        outbound("store_response_commitment")
    }

    fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
        vec![Box::new(Sp1BeefyConsensusClient {
            sp1_vkey_hash: self.sp1_vkey_hash,
            consensus_client_id: BEFY,
            commit_header_index: self.commit_header_index,
        })]
    }

    fn challenge_period(&self, _id: StateMachineId) -> Option<Duration> {
        Some(Duration::from_secs(0))
    }
    fn store_challenge_period(
        &self,
        _id: StateMachineId,
        _p: u64,
    ) -> Result<(), IsmpError> {
        outbound("store_challenge_period")
    }
    fn allowed_proxy(&self) -> Option<StateMachine> {
        None
    }
    fn unbonding_period(&self, _id: ConsensusStateId) -> Option<Duration> {
        Some(Duration::from_secs(self.unbonding_period_secs))
    }
    fn ismp_router(&self) -> Box<dyn IsmpRouter> {
        Box::new(NoopRouter)
    }
    fn previous_commitment_height(&self, _id: StateMachineId) -> Option<u64> {
        None
    }
}

struct NoopRouter;
impl IsmpRouter for NoopRouter {
    fn module_for_id(
        &self,
        _b: Vec<u8>,
    ) -> core::result::Result<Box<dyn ismp::module::IsmpModule>, anyhow::Error> {
        Ok(Box::new(SolanaCpiModule))
    }
}

fn fresh_consensus_message() -> ConsensusMessage {
    ConsensusMessage {
        consensus_proof: wire_proof_bytes(),
        consensus_state_id: BEFY,
        signer: b"unit-test-relayer".to_vec(),
    }
}

#[test]
fn sp1_beefy_consensus_client_verifies_real_fixture() {
    let host = RecordingHost::new(trusted_state_bytes());
    let client = Sp1BeefyConsensusClient {
        sp1_vkey_hash: sp1_vkey_hash(),
        consensus_client_id: BEFY,
        commit_header_index: Some(0),
    };

    let (new_state, intermediates) = client
        .verify_consensus(&host, BEFY, trusted_state_bytes(), wire_proof_bytes())
        .expect("real BEEFY fixture verifies");

    assert!(!new_state.is_empty());
    assert_eq!(intermediates.len(), 1);

    let pre = BeefyConsensusState::decode(&mut trusted_state_bytes().as_slice()).unwrap();
    let post = BeefyConsensusState::decode(&mut new_state.as_slice()).unwrap();
    assert!(post.latest_beefy_height > pre.latest_beefy_height);
}

#[test]
fn handle_incoming_consensus_advances_state_and_records_commitment() {
    let host = RecordingHost::new(trusted_state_bytes());
    ismp::handlers::handle_incoming_message(&host, Message::Consensus(fresh_consensus_message()))
        .unwrap();

    assert_eq!(host.stored_consensus_states.borrow().len(), 1);
    assert_eq!(host.stored_state_commitments.borrow().len(), 1);
    assert_eq!(host.stored_latest_heights.borrow().len(), 1);
    let new_state = host.stored_consensus_states.borrow()[0].clone();
    let post = BeefyConsensusState::decode(&mut new_state.as_slice()).unwrap();
    let pre = BeefyConsensusState::decode(&mut trusted_state_bytes().as_slice()).unwrap();
    assert!(post.latest_beefy_height > pre.latest_beefy_height);
}

#[test]
fn handle_incoming_consensus_rejects_when_frozen() {
    let mut host = RecordingHost::new(trusted_state_bytes());
    host.frozen = true;

    let err = ismp::handlers::handle_incoming_message(
        &host,
        Message::Consensus(fresh_consensus_message()),
    )
    .unwrap_err();
    assert!(format!("{err:?}").to_lowercase().contains("frozen"));
    assert!(host.stored_consensus_states.borrow().is_empty());
}

#[test]
fn handle_incoming_consensus_rejects_when_expired() {
    let mut host = RecordingHost::new(trusted_state_bytes());
    host.unbonding_period_secs = 60;
    *host.consensus_last_updated_secs.borrow_mut() = host.now_unix_secs - 3600;

    let err = ismp::handlers::handle_incoming_message(
        &host,
        Message::Consensus(fresh_consensus_message()),
    )
    .unwrap_err();
    let s = format!("{err:?}").to_lowercase();
    assert!(s.contains("unbonding") || s.contains("expired"));
    assert!(host.stored_consensus_states.borrow().is_empty());
}

#[test]
fn commitment_is_recoverable_via_state_machine_commitment_after_handle() {
    let host = RecordingHost::new(trusted_state_bytes());
    ismp::handlers::handle_incoming_message(&host, Message::Consensus(fresh_consensus_message()))
        .unwrap();

    let (h, stored) = host.stored_state_commitments.borrow()[0].clone();
    let read_back = host.state_machine_commitment(h).unwrap();
    assert_eq!(read_back.state_root, stored.state_root);
    assert_eq!(read_back.timestamp, stored.timestamp);
}

const SOURCE_PARA: u32 = 2000;
const PROOF_HEIGHT: u64 = 100;

/// `generate_trie_proof` produces a compact proof for `verify_trie_proof`;
/// the on-chain verifier rebuilds a `MemoryDB` and runs `trie.get`, which
/// needs full RLP-encoded nodes — captured here via a `Recorder`.
fn build_single_key_proof(key: &[u8], value: &[u8]) -> ([u8; 32], Vec<u8>) {
    type Layout = LayoutV0<Blake2Hasher>;

    let mut db = MemoryDB::<Blake2Hasher>::default();
    let mut root = empty_trie_root::<Layout>();
    {
        let mut trie = TrieDBMutBuilder::<Layout>::new(&mut db, &mut root).build();
        trie.insert(key, value).expect("trie insert");
    }

    let recorder = Recorder::<Blake2Hasher>::default();
    {
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<Layout>::new(&db, &root)
            .with_recorder(&mut trie_recorder)
            .build();
        let read = trie.get(key).expect("trie get").expect("value present");
        assert_eq!(read.as_slice(), value);
    }
    let storage_proof = recorder.drain_storage_proof();
    let proof_nodes: Vec<Vec<u8>> = storage_proof.into_iter_nodes().collect();
    let wire = (key.to_vec(), proof_nodes).encode();
    (root.0, wire)
}

fn synthetic_post_request() -> PostRequest {
    PostRequest {
        source: StateMachine::Polkadot(SOURCE_PARA),
        dest: SOLANA_STATE_MACHINE,
        nonce: 7,
        from: vec![0xab; 20],
        // Solana program IDs are 32 bytes.
        to: vec![0xcd; 32],
        // 0 ⇒ no timeout per ismp convention.
        timeout_timestamp: 0,
        body: b"hello-from-evm".to_vec(),
    }
}

fn seed_state_commitment(host: &RecordingHost, state_root: [u8; 32]) -> StateMachineHeight {
    let height = StateMachineHeight {
        id: StateMachineId {
            state_id: StateMachine::Polkadot(SOURCE_PARA),
            consensus_state_id: BEFY,
        },
        height: PROOF_HEIGHT,
    };
    host.state_commitments.borrow_mut().insert(
        (height.id, height.height),
        (
            IsmpStateCommitment {
                timestamp: host.now_unix_secs as u64,
                overlay_root: None,
                state_root: H256::from(state_root),
            },
            Duration::from_secs(host.now_unix_secs as u64),
        ),
    );
    height
}

#[test]
fn handle_incoming_post_request_records_receipt_and_dispatches() {
    let (state_root, wire_proof) =
        build_single_key_proof(b"req-storage-key", b"req-storage-value");
    let host = RecordingHost::new(trusted_state_bytes());
    let height = seed_state_commitment(&host, state_root);

    let post = synthetic_post_request();
    let expected_commitment = hash_request::<RecordingHost>(&Request::Post(post.clone()));

    let request_msg = RequestMessage {
        requests: vec![post],
        proof: Proof { height, proof: wire_proof },
        signer: b"unit-test-relayer".to_vec(),
    };

    ismp::handlers::handle_incoming_message(&host, Message::Request(request_msg)).unwrap();

    let receipts = host.stored_request_receipts.borrow();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0], expected_commitment);
    assert!(host.deleted_request_receipts.borrow().is_empty());
}

#[test]
fn handle_incoming_post_request_rejects_on_bad_state_root() {
    let (good_root, wire_proof) =
        build_single_key_proof(b"req-storage-key", b"req-storage-value");
    let mut bogus_root = good_root;
    bogus_root[0] ^= 0xff;

    let host = RecordingHost::new(trusted_state_bytes());
    let height = seed_state_commitment(&host, bogus_root);

    let request_msg = RequestMessage {
        requests: vec![synthetic_post_request()],
        proof: Proof { height, proof: wire_proof },
        signer: b"unit-test-relayer".to_vec(),
    };

    let err = ismp::handlers::handle_incoming_message(&host, Message::Request(request_msg))
        .unwrap_err();
    let s = format!("{err:?}").to_lowercase();
    assert!(s.contains("membership") || s.contains("storage proof") || s.contains("verification"));
    assert!(host.stored_request_receipts.borrow().is_empty());
}

#[test]
fn handle_incoming_post_request_rejects_wrong_destination() {
    let (state_root, wire_proof) =
        build_single_key_proof(b"req-storage-key", b"req-storage-value");
    let host = RecordingHost::new(trusted_state_bytes());
    let height = seed_state_commitment(&host, state_root);

    let mut post = synthetic_post_request();
    post.dest = StateMachine::Polkadot(9999);

    let request_msg = RequestMessage {
        requests: vec![post],
        proof: Proof { height, proof: wire_proof },
        signer: b"unit-test-relayer".to_vec(),
    };
    let err = ismp::handlers::handle_incoming_message(&host, Message::Request(request_msg))
        .unwrap_err();
    let s = format!("{err:?}").to_lowercase();
    assert!(s.contains("destination") || s.contains("invalid"));
    assert!(host.stored_request_receipts.borrow().is_empty());
}
