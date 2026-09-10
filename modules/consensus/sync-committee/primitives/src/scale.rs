// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! SCALE codec for the consensus types that carry an ssz container.
//!
//! `SyncCommittee` and `SyncAggregate` are the only types crossing the runtime boundary that hold
//! one, and the orphan rule keeps the impls for `FixedVector` and `BitVector` out of this crate.
//! Carrying those impls in a fork of the ssz crates is the one change upstream will never take,
//! since it puts a Substrate dependency in an ssz library, so the two owning structs implement
//! codec themselves and write their container fields as the plain `Vec` the wire already uses.
//!
//! # Wire compatibility
//!
//! These encodings are byte for byte what the derives produced while the containers were codec
//! aware, which in turn matched `ssz-rs`:
//!
//! * a `FixedVector<T, N>` encodes as its inner `Vec<T>`
//! * a `BitVector<N>` encodes as `Vec<bool>`, one byte per bit, not as its packed ssz bytes
//!
//! Decoding re-applies each container's length rule, so a payload that would produce a wrongly
//! sized committee or an over long bitfield is rejected at the boundary rather than surviving as
//! an invalid value. See [`crate::ssz::Root`] for the same reasoning applied to a 32 byte root.

use crate::{
	consensus_types::{SyncAggregate, SyncCommittee},
	constants::{BlsPublicKey, BlsSignature},
};
use alloc::vec::Vec;
use codec::{Decode, Encode, Error, Input, Output};
use ssz_types::{typenum::Unsigned, BitVector, FixedVector};

impl<SYNC_COMMITTEE_SIZE: Unsigned> Encode for SyncCommittee<SYNC_COMMITTEE_SIZE> {
	fn encode_to<O: Output + ?Sized>(&self, dest: &mut O) {
		// `[T]` and `Vec<T>` share a SCALE encoding, so going via the slice avoids a clone.
		let keys: &[BlsPublicKey] = &self.public_keys;
		keys.encode_to(dest);
		self.aggregate_public_key.encode_to(dest);
	}
}

impl<SYNC_COMMITTEE_SIZE: Unsigned> Decode for SyncCommittee<SYNC_COMMITTEE_SIZE> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let keys = Vec::<BlsPublicKey>::decode(input)?;
		let public_keys = FixedVector::new(keys)
			.map_err(|_| Error::from("SyncCommittee: wrong number of public keys"))?;
		let aggregate_public_key = BlsPublicKey::decode(input)?;
		Ok(Self { public_keys, aggregate_public_key })
	}
}

impl<SYNC_COMMITTEE_SIZE: Unsigned> codec::EncodeLike for SyncCommittee<SYNC_COMMITTEE_SIZE> {}

impl<SYNC_COMMITTEE_SIZE: Unsigned + Clone> Encode for SyncAggregate<SYNC_COMMITTEE_SIZE> {
	fn encode_to<O: Output + ?Sized>(&self, dest: &mut O) {
		// One byte per bit. Wasteful, but it is what previously encoded updates contain.
		self.sync_committee_bits.iter().collect::<Vec<bool>>().encode_to(dest);
		self.sync_committee_signature.encode_to(dest);
	}
}

impl<SYNC_COMMITTEE_SIZE: Unsigned + Clone> Decode for SyncAggregate<SYNC_COMMITTEE_SIZE> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let bits: Vec<bool> = Decode::decode(input)?;
		if bits.len() > SYNC_COMMITTEE_SIZE::to_usize() {
			return Err(Error::from("SyncAggregate: too many participation bits"));
		}
		// A fixed bitfield is always `N` bits wide, so a short encoding leaves the tail unset.
		let mut sync_committee_bits = BitVector::new();
		for (i, bit) in bits.into_iter().enumerate() {
			sync_committee_bits
				.set(i, bit)
				.map_err(|_| Error::from("SyncAggregate: bit index out of range"))?;
		}
		let sync_committee_signature = BlsSignature::decode(input)?;
		Ok(Self { sync_committee_bits, sync_committee_signature })
	}
}

impl<SYNC_COMMITTEE_SIZE: Unsigned + Clone> codec::EncodeLike
	for SyncAggregate<SYNC_COMMITTEE_SIZE>
{
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::constants::SYNC_COMMITTEE_SIZE;
	use ssz_types::typenum::U4;

	fn committee<N: Unsigned>(n: usize) -> SyncCommittee<N> {
		SyncCommittee {
			public_keys: FixedVector::new(vec![BlsPublicKey::default(); n]).unwrap(),
			aggregate_public_key: BlsPublicKey::default(),
		}
	}

	/// The reason this module exists. A committee has to encode as its plain `Vec` of keys
	/// followed by the aggregate, which is what the derives produced while `FixedVector` itself
	/// was codec aware, and what every stored consensus state contains.
	#[test]
	fn sync_committee_encodes_as_its_keys_then_aggregate() {
		let c = committee::<U4>(4);

		let mut expected = vec![BlsPublicKey::default(); 4].encode();
		expected.extend(BlsPublicKey::default().encode());

		assert_eq!(c.encode(), expected);
	}

	#[test]
	fn sync_committee_round_trips() {
		let c = committee::<U4>(4);
		assert_eq!(SyncCommittee::<U4>::decode(&mut &c.encode()[..]).unwrap(), c);
	}

	/// A committee of the wrong size must not survive decoding. The ssz root would not necessarily
	/// catch it, so the length rule has to be re-applied here.
	#[test]
	fn decoding_rejects_a_wrongly_sized_committee() {
		let mut too_few = vec![BlsPublicKey::default(); 3].encode();
		too_few.extend(BlsPublicKey::default().encode());
		assert!(SyncCommittee::<U4>::decode(&mut &too_few[..]).is_err());

		let mut too_many = vec![BlsPublicKey::default(); 5].encode();
		too_many.extend(BlsPublicKey::default().encode());
		assert!(SyncCommittee::<U4>::decode(&mut &too_many[..]).is_err());
	}

	/// Bits go out as one byte per bit, not as packed ssz bytes.
	#[test]
	fn sync_aggregate_bits_encode_as_vec_of_bool() {
		let mut bits = BitVector::<SYNC_COMMITTEE_SIZE>::new();
		bits.set(0, true).unwrap();
		bits.set(3, true).unwrap();

		let aggregate = SyncAggregate::<SYNC_COMMITTEE_SIZE> {
			sync_committee_bits: bits.clone(),
			sync_committee_signature: BlsSignature::default(),
		};

		let expected_bits: Vec<bool> = bits.iter().collect();
		assert_eq!(expected_bits.len(), SYNC_COMMITTEE_SIZE::to_usize());

		let mut expected = expected_bits.encode();
		expected.extend(BlsSignature::default().encode());
		assert_eq!(aggregate.encode(), expected);
	}

	#[test]
	fn sync_aggregate_round_trips() {
		let mut bits = BitVector::<SYNC_COMMITTEE_SIZE>::new();
		bits.set(1, true).unwrap();

		let aggregate = SyncAggregate::<SYNC_COMMITTEE_SIZE> {
			sync_committee_bits: bits,
			sync_committee_signature: BlsSignature::default(),
		};

		let decoded =
			SyncAggregate::<SYNC_COMMITTEE_SIZE>::decode(&mut &aggregate.encode()[..]).unwrap();
		assert_eq!(decoded, aggregate);
	}

	#[test]
	fn decoding_rejects_an_over_long_bitfield() {
		let too_many = vec![true; SYNC_COMMITTEE_SIZE::to_usize() + 1];
		let mut bytes = too_many.encode();
		bytes.extend(BlsSignature::default().encode());
		assert!(SyncAggregate::<SYNC_COMMITTEE_SIZE>::decode(&mut &bytes[..]).is_err());
	}
}
