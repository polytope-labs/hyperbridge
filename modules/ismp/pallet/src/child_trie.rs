//! pallet Ismp child trie

use core::marker::PhantomData;
use frame_support::storage::{child, storage_prefix};
use sp_core::{storage::ChildInfo, H256};

use crate::{dispatcher::LeafMetadata, Config, ResponseReceipt};

/// Commitments for outgoing requests
/// The key is the request commitment
pub struct RequestCommitments<T: Config>(PhantomData<T>);

/// Receipts for incoming requests
/// The key is the request commitment
pub struct RequestReceipts<T: Config>(PhantomData<T>);

/// Commitments for outgoing responses
/// The key is the response commitment
pub struct ResponseCommitments<T: Config>(PhantomData<T>);

/// Receipts for incoming responses
/// The key is the request commitment
pub struct ResponseReceipts<T: Config>(PhantomData<T>);

const PALLET_NAME: &'static str = "ismp";

impl<T: Config> RequestCommitments<T> {
    /// Returns the hashed storage key
    pub fn storage_key(key: H256) -> Vec<u8> {
        let mut full_key =
            storage_prefix(PALLET_NAME.as_bytes(), "RequestCommitments".as_bytes()).to_vec();
        full_key.extend_from_slice(&key.0);
        full_key
    }

    /// Get the provided key from the child trie
    pub fn get(key: H256) -> Option<LeafMetadata<T>> {
        child::get(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Insert the key and value into the child trie
    pub fn insert(key: H256, meta: LeafMetadata<T>) {
        child::put(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key), &meta);
    }

    /// Remove the key from the child trie
    pub fn remove(key: H256) {
        child::kill(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Return true if key is contained in child trie
    pub fn contains_key(key: H256) -> bool {
        child::exists(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }
}

impl<T: Config> ResponseCommitments<T> {
    /// Returns the hashed storage key
    pub fn storage_key(key: H256) -> Vec<u8> {
        let mut full_key =
            storage_prefix(PALLET_NAME.as_bytes(), "ResponseCommitments".as_bytes()).to_vec();
        full_key.extend_from_slice(&key.0);
        full_key
    }

    /// Get the provided key from the child trie
    pub fn get(key: H256) -> Option<LeafMetadata<T>> {
        child::get(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Insert the key and value into the child trie
    pub fn insert(key: H256, meta: LeafMetadata<T>) {
        child::put(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key), &meta);
    }

    /// Remove the key from the child trie
    pub fn remove(key: H256) {
        child::kill(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Return true if key is contained in child trie
    pub fn contains_key(key: H256) -> bool {
        child::exists(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }
}

impl<T: Config> RequestReceipts<T> {
    /// Returns the hashed storage key
    pub fn storage_key(key: H256) -> Vec<u8> {
        let mut full_key =
            storage_prefix(PALLET_NAME.as_bytes(), "RequestReceipts".as_bytes()).to_vec();
        full_key.extend_from_slice(&key.0);
        full_key
    }

    /// Get the provided key from the child trie
    pub fn get(key: H256) -> Option<Vec<u8>> {
        child::get(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Insert the key and value into the child trie
    pub fn insert(key: H256, relayer: &[u8]) {
        child::put(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key), &relayer);
    }

    /// Remove the key from the child trie
    pub fn remove(key: H256) {
        child::kill(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Return true if key is contained in child trie
    pub fn contains_key(key: H256) -> bool {
        child::exists(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }
}

impl<T: Config> ResponseReceipts<T> {
    /// Returns the hashed storage key
    pub fn storage_key(key: H256) -> Vec<u8> {
        let mut full_key =
            storage_prefix(PALLET_NAME.as_bytes(), "ResponseReceipts".as_bytes()).to_vec();
        full_key.extend_from_slice(&key.0);
        full_key
    }

    /// Get the provided key from the child trie
    pub fn get(key: H256) -> Option<ResponseReceipt> {
        child::get(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Insert the key and value into the child trie
    pub fn insert(key: H256, receipt: ResponseReceipt) {
        child::put(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key), &receipt);
    }

    /// Remove the key from the child trie
    pub fn remove(key: H256) {
        child::kill(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }

    /// Return true if key is contained in child trie
    pub fn contains_key(key: H256) -> bool {
        child::exists(&ChildInfo::new_default(T::PALLET_PREFIX), &Self::storage_key(key))
    }
}
