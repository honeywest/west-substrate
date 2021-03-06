// Copyright 2017-2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Consensus module for runtime; manages the authority set ready for the native code.

#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
#[macro_use]
extern crate sr_std as rstd;

#[macro_use]
extern crate srml_support as runtime_support;

extern crate parity_codec;
#[macro_use]
extern crate parity_codec_derive;

extern crate parity_codec as codec;
extern crate sr_primitives as primitives;
extern crate srml_system as system;
extern crate substrate_primitives;

#[cfg(test)]
extern crate sr_io as runtime_io;

use parity_codec::Encode;
use primitives::traits::{Block as BlockT, MaybeSerializeDebug, Member, ProvideInherent};
use primitives::CheckInherentError;
use rstd::prelude::*;
use rstd::result;
use runtime_support::storage::unhashed::StorageVec;
use runtime_support::storage::StorageValue;
use runtime_support::{storage, Parameter};
use substrate_primitives::storage::well_known_keys;
use system::{ensure_inherent, ensure_signed};

mod mock;
mod tests;

struct AuthorityStorageVec<S: codec::Codec + Default>(rstd::marker::PhantomData<S>);
impl<S: codec::Codec + Default> StorageVec for AuthorityStorageVec<S> {
    type Item = S;
    const PREFIX: &'static [u8] = well_known_keys::AUTHORITY_PREFIX;
}

pub type KeyValue = (Vec<u8>, Vec<u8>);

/// Handling offline validator reports in a generic way.
pub trait OnOfflineReport<Offline> {
    fn handle_report(offline: Offline);
}

impl<T> OnOfflineReport<T> for () {
    fn handle_report(_: T) {}
}

/// Describes the offline-reporting extrinsic.
pub trait InherentOfflineReport {
    /// The report data type passed to the runtime during block authorship.
    type Inherent: codec::Codec + Parameter;

    /// Whether an inherent is empty and doesn't need to be included.
    fn is_empty(inherent: &Self::Inherent) -> bool;

    /// Handle the report.
    fn handle_report(report: Self::Inherent);

    /// Whether two reports are compatible.
    fn check_inherent(
        contained: &Self::Inherent,
        expected: &Self::Inherent,
    ) -> Result<(), &'static str>;
}

impl InherentOfflineReport for () {
    type Inherent = ();

    fn is_empty(_inherent: &()) -> bool {
        true
    }
    fn handle_report(_: ()) {}
    fn check_inherent(_: &(), _: &()) -> Result<(), &'static str> {
        Err("Explicit reporting not allowed")
    }
}

/// A variant of the `OfflineReport` which is useful for instant-finality blocks.
///
/// This assumes blocks are only finalized
pub struct InstantFinalityReportVec<T>(::rstd::marker::PhantomData<T>);

impl<T: OnOfflineReport<Vec<u32>>> InherentOfflineReport for InstantFinalityReportVec<T> {
    type Inherent = Vec<u32>;

    fn is_empty(inherent: &Self::Inherent) -> bool {
        inherent.is_empty()
    }

    fn handle_report(report: Vec<u32>) {
        T::handle_report(report)
    }

    fn check_inherent(
        contained: &Self::Inherent,
        expected: &Self::Inherent,
    ) -> Result<(), &'static str> {
        contained.iter().try_for_each(|n| {
            if !expected.contains(n) {
                Err("Node we believe online marked offline")
            } else {
                Ok(())
            }
        })
    }
}

pub type Log<T> = RawLog<<T as Trait>::SessionKey>;

/// A logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<SessionKey> {
    /// Authorities set has been changed. Contains the new set of authorities.
    AuthoritiesChange(Vec<SessionKey>),
}

impl<SessionKey: Member> RawLog<SessionKey> {
    /// Try to cast the log entry as AuthoritiesChange log entry.
    pub fn as_authorities_change(&self) -> Option<&[SessionKey]> {
        match *self {
            RawLog::AuthoritiesChange(ref item) => Some(item),
        }
    }
}

// Implementation for tests outside of this crate.
#[cfg(any(feature = "std", test))]
impl<N> From<RawLog<N>> for primitives::testing::DigestItem
where
    N: Into<u64>,
{
    fn from(log: RawLog<N>) -> primitives::testing::DigestItem {
        match log {
            RawLog::AuthoritiesChange(authorities) => {
                primitives::generic::DigestItem::AuthoritiesChange::<substrate_primitives::H256, u64>(
                    authorities.into_iter().map(Into::into).collect(),
                )
            }
        }
    }
}

pub trait Trait: system::Trait {
    /// The allowed extrinsic position for `note_offline` inherent.
    const NOTE_OFFLINE_POSITION: u32;

    /// Type for all log entries of this module.
    type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;

    type SessionKey: Parameter + Default + MaybeSerializeDebug;
    /// Defines the offline-report type of the trait.
    /// Set to `()` if offline-reports aren't needed for this runtime.
    type InherentOfflineReport: InherentOfflineReport;
}

decl_storage! {
    trait Store for Module<T: Trait> as Consensus {
        // Authorities set actual at the block execution start. IsSome only if
        // the set has been changed.
        OriginalAuthorities: Option<Vec<T::SessionKey>>;
    }
    add_extra_genesis {
        config(authorities): Vec<T::SessionKey>;
        #[serde(with = "substrate_primitives::bytes")]
        config(code): Vec<u8>;

        build(|storage: &mut primitives::StorageMap, _: &mut primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
            use codec::{Encode, KeyedVec};

            let auth_count = config.authorities.len() as u32;
            config.authorities.iter().enumerate().for_each(|(i, v)| {
                storage.insert((i as u32).to_keyed_vec(well_known_keys::AUTHORITY_PREFIX), v.encode());
            });
            storage.insert(well_known_keys::AUTHORITY_COUNT.to_vec(), auth_count.encode());
            storage.insert(well_known_keys::CODE.to_vec(), config.code.clone());
        });
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Report some misbehaviour.
        fn report_misbehavior(origin, _report: Vec<u8>) {
            ensure_signed(origin)?;
            // TODO: requires extension trait.
        }

        /// Note the previous block's validator missed their opportunity to propose a block.
        fn note_offline(origin, offline: <T::InherentOfflineReport as InherentOfflineReport>::Inherent) {
            ensure_inherent(origin)?;

            assert!(
                <system::Module<T>>::extrinsic_index() == Some(T::NOTE_OFFLINE_POSITION),
                "note_offline extrinsic must be at position {} in the block",
                T::NOTE_OFFLINE_POSITION
            );

            T::InherentOfflineReport::handle_report(offline);
        }

        /// Make some on-chain remark.
        fn remark(origin, _remark: Vec<u8>) {
            ensure_signed(origin)?;
        }

        /// Set the number of pages in the WebAssembly environment's heap.
        fn set_heap_pages(pages: u64) {
            storage::unhashed::put_raw(well_known_keys::HEAP_PAGES, &pages.encode());
        }

        /// Set the new code.
        pub fn set_code(new: Vec<u8>) {
            storage::unhashed::put_raw(well_known_keys::CODE, &new);
        }

        /// Set some items of storage.
        fn set_storage(items: Vec<KeyValue>) {
            for i in &items {
                storage::unhashed::put_raw(&i.0, &i.1);
            }
        }

        fn on_finalise() {
            if let Some(original_authorities) = <OriginalAuthorities<T>>::take() {
                let current_authorities = AuthorityStorageVec::<T::SessionKey>::items();
                if current_authorities != original_authorities {
                    Self::deposit_log(RawLog::AuthoritiesChange(current_authorities));
                }
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Get the current set of authorities. These are the session keys.
    pub fn authorities() -> Vec<T::SessionKey> {
        AuthorityStorageVec::<T::SessionKey>::items()
    }

    /// Set the current set of authorities' session keys.
    ///
    /// Called by `next_session` only.
    pub fn set_authorities(authorities: &[T::SessionKey]) {
        let current_authorities = AuthorityStorageVec::<T::SessionKey>::items();
        if current_authorities != authorities {
            Self::save_original_authorities(Some(current_authorities));
            AuthorityStorageVec::<T::SessionKey>::set_items(authorities);
        }
    }

    /// Set a single authority by index.
    pub fn set_authority(index: u32, key: &T::SessionKey) {
        let current_authority = AuthorityStorageVec::<T::SessionKey>::item(index);
        if current_authority != *key {
            Self::save_original_authorities(None);
            AuthorityStorageVec::<T::SessionKey>::set_item(index, key);
        }
    }

    /// Save original authorities set.
    fn save_original_authorities(current_authorities: Option<Vec<T::SessionKey>>) {
        if OriginalAuthorities::<T>::get().is_some() {
            // if we have already saved original set before, do not overwrite
            return;
        }

        <OriginalAuthorities<T>>::put(
            current_authorities.unwrap_or_else(|| AuthorityStorageVec::<T::SessionKey>::items()),
        );
    }

    /// Deposit one of this module's logs.
    fn deposit_log(log: Log<T>) {
        <system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Inherent = <T::InherentOfflineReport as InherentOfflineReport>::Inherent;
    type Call = Call<T>;

    fn create_inherent_extrinsics(data: Self::Inherent) -> Vec<(u32, Self::Call)> {
        if <T::InherentOfflineReport as InherentOfflineReport>::is_empty(&data) {
            vec![]
        } else {
            vec![(T::NOTE_OFFLINE_POSITION, Call::note_offline(data))]
        }
    }

    fn check_inherent<Block: BlockT, F: Fn(&Block::Extrinsic) -> Option<&Self::Call>>(
        block: &Block,
        expected: Self::Inherent,
        extract_function: &F,
    ) -> result::Result<(), CheckInherentError> {
        let noted_offline = block
            .extrinsics()
            .get(T::NOTE_OFFLINE_POSITION as usize)
            .and_then(|xt| match extract_function(&xt) {
                Some(Call::note_offline(ref x)) => Some(x),
                _ => None,
            });

        // REVIEW: perhaps we should be passing a `None` to check_inherent.
        if let Some(noted_offline) = noted_offline {
            <T::InherentOfflineReport as InherentOfflineReport>::check_inherent(
                &noted_offline,
                &expected,
            )
            .map_err(|e| CheckInherentError::Other(e.into()))?;
        }

        Ok(())
    }
}
