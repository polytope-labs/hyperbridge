// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Schema for MMR-gadget state persisted in the aux-db.

use crate::LOG_TARGET;
use codec::{Decode, Encode};
use log::{info, trace};
use polkadot_sdk::*;
use sc_client_api::backend::AuxStore;
use sp_blockchain::{Error as ClientError, Result as ClientResult};
use sp_runtime::traits::{Block, NumberFor};

const VERSION_KEY: &[u8] = b"mmr_auxschema_version";
const GADGET_STATE: &[u8] = b"mmr_gadget_state";

const CURRENT_VERSION: u32 = 1;
pub(crate) type PersistedState<B> = NumberFor<B>;

pub(crate) fn write_current_version<B: AuxStore>(backend: &B) -> ClientResult<()> {
	info!(target: LOG_TARGET, "write aux schema version {:?}", CURRENT_VERSION);
	AuxStore::insert_aux(backend, &[(VERSION_KEY, CURRENT_VERSION.encode().as_slice())], &[])
}

/// Write gadget state.
pub(crate) fn write_gadget_state<B: Block, BE: AuxStore>(
	backend: &BE,
	state: &PersistedState<B>,
) -> ClientResult<()> {
	trace!(target: LOG_TARGET, "Storing best canonicalized block {:?}", state);
	backend.insert_aux(&[(GADGET_STATE, state.encode().as_slice())], &[])
}

fn load_decode<B: AuxStore, T: Decode>(backend: &B, key: &[u8]) -> ClientResult<Option<T>> {
	match backend.get_aux(key)? {
		None => Ok(None),
		Some(t) => T::decode(&mut &t[..])
			.map_err(|e| ClientError::Backend(format!("MMR aux DB is corrupted: {}", e)))
			.map(Some),
	}
}

/// Load persistent data from backend.
pub(crate) fn load_state<B, BE>(backend: &BE) -> ClientResult<Option<PersistedState<B>>>
where
	B: Block,
	BE: AuxStore,
{
	let version: Option<u32> = load_decode(backend, VERSION_KEY)?;

	match version {
		None => (),
		Some(1) => return load_decode::<_, PersistedState<B>>(backend, GADGET_STATE),
		other =>
			return Err(ClientError::Backend(format!("Unsupported MMR aux DB version: {:?}", other))),
	}

	// No persistent state found in DB.
	Ok(None)
}

/// Load or initialize persistent data from backend.
pub(crate) fn load_or_init_state<B, BE>(
	backend: &BE,
	default: NumberFor<B>,
) -> sp_blockchain::Result<NumberFor<B>>
where
	B: Block,
	BE: AuxStore,
{
	// Initialize gadget best_canon from AUX DB or from pallet genesis.
	if let Some(best) = load_state::<B, BE>(backend)? {
		info!(target: LOG_TARGET, "Loading MMR best canonicalized state from db: {:?}.", best);
		Ok(best)
	} else {
		info!(
			target: LOG_TARGET,
			"Loading MMR from pallet genesis on what appears to be the first startup: {:?}.",
			default
		);
		write_current_version(backend)?;
		write_gadget_state::<B, BE>(backend, &default)?;
		Ok(default)
	}
}
