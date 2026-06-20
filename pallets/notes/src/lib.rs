//! # Template Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Substrate template
//! nodes as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! Learn more about FRAME macros [here](https://docs.substrate.io/reference/frame-macros/).
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// FRAME pallets require their own "mock runtimes" to be able to run unit tests. This module
// contains a mock runtime specific for testing this pallet's functionality.
#[cfg(test)]
mod mock;

// This module contains the unit tests for this pallet.
// Learn about pallet unit testing here: https://docs.substrate.io/test/unit-testing/
#[cfg(test)]
mod tests;

// Every callable function or "dispatchable" a pallet exposes must have weight values that correctly
// estimate a dispatchable's execution time. The benchmarking module is used to calculate weights
// for each dispatchable and generates this pallet's weight.rs file. Learn more about benchmarking here: https://docs.substrate.io/test/benchmark/
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
    use scale_info::prelude::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use frame_system::pallet_prelude::BlockNumberFor;

    pub type NoteId = u64;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
        + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::constant]
        type MaxNoteLength: Get<u32>;

        #[pallet::constant]
        type MaxNotesPerAccount: Get<u32>;
    }


    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Note<T: Config> {
        pub content: BoundedVec<u8, T::MaxNoteLength>,
        pub created_at: Option<BlockNumberFor<T>>,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);


    #[pallet::storage]
    pub type NextNoteId<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, NoteId, ValueQuery>;

    #[pallet::storage]
    pub type Notes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        NoteId,
        Note<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type NoteCount<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            let current_version = StorageVersion::get::<Pallet<T>>();

            if current_version == StorageVersion::new(0) {
                Notes::<T>::translate_values::<BoundedVec<u8, T::MaxNoteLength>, _>(|old_content| {
                    Some(Note::<T> {
                        content: old_content,
                        created_at: None,
                    })
                });

                StorageVersion::new(1).put::<Pallet<T>>();

                return T::DbWeight::get().reads_writes(1, 1);
            }

            T::DbWeight::get().reads(1)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NoteCreated {
            owner: T::AccountId,
            note_id: NoteId,
        },
        NoteUpdated {
            owner: T::AccountId,
            note_id: NoteId,
        },
        NoteDeleted {
            owner: T::AccountId,
            note_id: NoteId,
        },
        NoteTransferred {
            from: T::AccountId,
            to: T::AccountId,
            from_note_id: NoteId,
            to_note_id: NoteId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NoteNotFound,
        NoteTooLong,
        NoteIdOverflow,
        TooManyNotes,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
        pub fn create_note(origin: OriginFor<T>, content: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let notes_count_by_owner = NoteCount::<T>::get(&who);
            ensure!(
                notes_count_by_owner < T::MaxNotesPerAccount::get(),
                Error::<T>::TooManyNotes
            );

            let bounded_content: BoundedVec<u8, T::MaxNoteLength> =
                content.try_into().map_err(|_| Error::<T>::NoteTooLong)?;

            let note_id = NextNoteId::<T>::get(&who);

            let next_id = note_id
                .checked_add(1)
                .ok_or(Error::<T>::NoteIdOverflow)?;

            let note = Note::<T> {
                content: bounded_content,
                created_at: Some(frame_system::Pallet::<T>::block_number()),
            };

            Notes::<T>::insert(&who, note_id, note);
            NextNoteId::<T>::insert(&who, next_id);
            NoteCount::<T>::insert(&who, notes_count_by_owner + 1);

            Self::deposit_event(Event::NoteCreated {
                owner: who,
                note_id,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_note(
            origin: OriginFor<T>,
            note_id: NoteId,
            content: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                Notes::<T>::contains_key(&who, note_id),
                Error::<T>::NoteNotFound
            );

            let bounded_content: BoundedVec<u8, T::MaxNoteLength> =
                content.try_into().map_err(|_| Error::<T>::NoteTooLong)?;

            Notes::<T>::try_mutate(&who, note_id, |maybe_note| -> DispatchResult {
                let note = maybe_note.as_mut().ok_or(Error::<T>::NoteNotFound)?;

                note.content = bounded_content;

                Ok(())
            })?;

            Self::deposit_event(Event::NoteUpdated {
                owner: who,
                note_id,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn delete_note(origin: OriginFor<T>, note_id: NoteId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let deleted = Notes::<T>::take(&who, note_id);

            ensure!(deleted.is_some(), Error::<T>::NoteNotFound);

            NoteCount::<T>::mutate(&who, |count| {
                *count = count.saturating_sub(1);
            });

            Self::deposit_event(Event::NoteDeleted {
                owner: who,
                note_id,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(6, 5))]
        pub fn transfer_note(origin: OriginFor<T>, to: T::AccountId, note_id: NoteId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            if !Notes::<T>::contains_key(&who, note_id){
                return Err(Error::<T>::NoteNotFound)?
            }
            let note = Notes::<T>::take(&who, note_id).ok_or(Error::<T>::NoteNotFound)?;

            let notes_count_by_recipient = NoteCount::<T>::get(&to);
            ensure!(
                notes_count_by_recipient < T::MaxNotesPerAccount::get(),
                Error::<T>::TooManyNotes
            );

            let recipient_note_id = NextNoteId::<T>::get(&to);
            let next_recipient_note_id = recipient_note_id
                .checked_add(1)
                .ok_or(Error::<T>::NoteIdOverflow)?;

            Notes::<T>::insert(&to, recipient_note_id, note);
            NextNoteId::<T>::insert(&to, next_recipient_note_id);
            NoteCount::<T>::mutate(&to, |count| {
                *count = count.saturating_add(1);
            });

            NoteCount::<T>::mutate(&who, |count| {
                *count = count.saturating_sub(1);
            });

            Self::deposit_event(Event::NoteTransferred {
                from: who.clone(),
                to,
                from_note_id: note_id,
                to_note_id: recipient_note_id,
            });

            Ok(())
        }
    }
}
