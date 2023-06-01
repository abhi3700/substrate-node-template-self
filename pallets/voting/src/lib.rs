//! # Voting Pallet
//!
//! A demonstration of a voting pallet.
//!
//! ## Overview
//!
//! The voting pallet provides functionality for voting on proposals created by
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

/// Simple index type for proposal counting.
pub type ProposalIndex = u32;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use crate::ProposalIndex;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + Get<ProposalIndex> + TypeInfo + Decode {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// TODO: Research if this macro is required.
		#[pallet::constant]
		type MaxStringLength: Get<u32>;
	}

	/// Storage for the available proposal index.
	#[pallet::storage]
	#[pallet::getter(fn proposal_index)]
	pub type ProposalIndexStorage<T: Config> = StorageValue<_, ProposalIndex>;

	/// A type for a single proposal.
	#[derive(Debug, Encode, Decode, Default, Clone, PartialEq, MaxEncodedLen, TypeInfo)]
	pub struct Proposal<T: Config> {
		name: BoundedVec<u8, T::MaxStringLength>,
		vote_count: u32,
		// TODO: Research for adding a timestamp type here.
		// Reference: https://stackoverflow.com/questions/68262293/substrate-frame-v2-how-to-use-pallet-timestamp
		vote_start_timestamp: Option<T::BlockNumber>,
	}

	/// Storage for all proposals.
	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProposalIndex,
		Proposal<T>,
	>;

	/// A type for a single voter.
	#[derive(Debug, Encode, Decode, Default, Clone, PartialEq, MaxEncodedLen, TypeInfo)]
	pub struct Voter<T: Config> {
		weight: u32,
		voted: bool,
		delegate: Option<T::AccountId>,
		proposal: ProposalIndex,
	}

	/// Storage for the voters
	#[pallet::storage]
	#[pallet::getter(fn voters)]
	pub type Voters<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Voter<T>>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a proposal is created.
		ProposalCreated { who: T::AccountId, proposal_id: ProposalIndex },
		/// Event emitted when a proposal is cancelled
		ProposalCancelled { who: T::AccountId, proposal_id: ProposalIndex },
		/// Event emitted when a proposal is voted on.
		ProposalVoted { who: T::AccountId, proposal_id: ProposalIndex },
		/// Event emitted when a voter delegates their vote.
		VoterDelegated { who: T::AccountId, to: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Already voted.
		AlreadyVoted,
		/// Zero proposal id.
		ZeroProposalId,
		/// Start timestamp must be in the future.
		StartTimestampMustBeInTheFuture,
		/// Proposal name cannot be empty.
		ProposalNameCannotBeEmpty,
		/// Proposal not created by caller.
		ProposalNotCreatedByCaller,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A dispatchable for creating a proposal. This function requires a signed transaction.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn create_proposal(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::MaxStringLength>,
			start_timestamp: T::BlockNumber,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			ensure!(name.len() > 0, Error::<T>::ProposalNameCannotBeEmpty);
			ensure!(
				start_timestamp > <frame_system::Pallet<T>>::block_number(),
				Error::<T>::StartTimestampMustBeInTheFuture
			);

			// NOTE: the proposal index is unwrapped as zero if it does not exist i.e. None.
			let proposal_id = <ProposalIndexStorage<T>>::get().unwrap_or(0);

			let proposal: Proposal<T> =
				Proposal { name, vote_count: 0, vote_start_timestamp: start_timestamp.into() };

			// Update storage for proposal
			<Proposals<T>>::insert(&who, proposal_id + 1, proposal);

			// Update storage for proposal index
			<ProposalIndexStorage<T>>::put(proposal_id + 1);

			// Emit an event.
			Self::deposit_event(Event::ProposalCreated { who, proposal_id });

			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// A dispatchable for cancelling a proposal. This function requires a signed transaction.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: ProposalIndex) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			ensure!(proposal_id > 0, Error::<T>::ZeroProposalId);

			// Check that the proposal exists.
			ensure!(
				<Proposals<T>>::contains_key(&who, proposal_id),
				Error::<T>::ProposalNotCreatedByCaller
			);

			// Remove the proposal from storage.
			<Proposals<T>>::remove(&who, proposal_id);

			// Emit an event.
			Self::deposit_event(Event::ProposalCancelled { who, proposal_id });

			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// A dispatchable for voting on a proposal. This function requires a signed transaction.
		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn vote(origin: OriginFor<T>, proposal_id: ProposalIndex) -> DispatchResult {
			// TODO: add logic for voting
			Ok(())
		}

		/// A dispatchable for delegating a vote. This function requires a signed transaction.
		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn delegate(origin: OriginFor<T>, to: T::AccountId) -> DispatchResult {
			// TODO: add logic for delegate voting
			Ok(())
		}
	}
}
