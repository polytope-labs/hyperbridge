#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use hash_db::Hasher;
use trie_db::TrieLayout;

mod node_codec;

/// Trie layout for EIP-1186 proof nodes.
#[derive(Default, Clone)]
pub struct EIP1186Layout<H>(PhantomData<H>);

impl<H: Hasher> TrieLayout for EIP1186Layout<H> {
    const USE_EXTENSION: bool = true;
    const ALLOW_EMPTY: bool = false;
    const MAX_INLINE_VALUE: Option<u32> = None;
    type Hash = H;
    type Codec = node_codec::RlpNodeCodec<H>;
}
