//! # Voting Pallet
//!
//! A demonstration of a voting pallet.
//!
//! ## Overview
//!
//! - The voting pallet provides functionality for voting on proposals created by an individual
//! account.
//! - The proposal creators cannot vote on their own proposal.
//! - The proposal creator can cancel their proposal before the voting period starts.
//! - The proposal voter can delegate their vote to another account. But need to check for self-delegation route.
//! - The proposal voter can vote on a proposal only once.
//! - The proposal voter can vote on a proposal only if the voting period has started & not ended yet.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### For Proposer
//!
//! - `create_proposal` - Create a new proposal. Add a new proposal if the existing proposal is done with voting.
//! - `cancel_proposal` - Cancel a proposal before the voting period starts.
//!
//! #### For Voter
//!
//! - `delegate_vote` - Delegate your vote to another account for a proposal if you have not voted yet.
//! 	But need to check for self-delegation route.
//! - `vote` - Vote on a proposal.
//!
//! ## Reference
//! - https://docs.soliditylang.org/en/latest/solidity-by-example.html#voting
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + TypeInfo {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// TODO: Research if this macro is required.
		#[pallet::constant]
		type MaxStringLength: Get<u32>;
	}

	/// Storage for the available proposal index.
	#[pallet::storage]
	#[pallet::getter(fn proposal_index)]
	pub type LastProposalIndex<T: Config> = StorageValue<_, u32>;

	/// A type for a single proposal.
	#[derive(Debug, Encode, Decode, Default, Clone, PartialEq, MaxEncodedLen, TypeInfo)]
	pub struct Proposal<T: Config> {
		proposer: T::AccountId,
		name: BoundedVec<u8, T::MaxStringLength>,
		vote_count: u32,
		// TODO: Research for adding a timestamp type here.
		// Reference: https://stackoverflow.com/questions/68262293/substrate-frame-v2-how-to-use-pallet-timestamp
		vote_start_timestamp: Option<T::BlockNumber>,
		vote_end_timestamp: Option<T::BlockNumber>,
	}

	/// Storage for all proposals.
	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u32, Proposal<T>>;

	/// A type for a single voter.
	#[derive(Debug, Encode, Decode, Clone, PartialEq, MaxEncodedLen, TypeInfo)]
	pub struct Voter<T: Config> {
		weight: u32,
		voted: bool,
		delegate: Option<T::AccountId>,
		proposal: u32,
	}

	// For each voter, we set the weight as 1 by default.
	impl<T: Config> Default for Voter<T> {
		fn default() -> Self {
			Self { weight: 1, voted: false, delegate: None, proposal: 0 }
		}
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
		ProposalCreated { who: T::AccountId, proposal_id: u32 },
		/// Event emitted when a proposal is cancelled
		ProposalCancelled { who: T::AccountId, proposal_id: u32 },
		/// Event emitted when a proposal is voted on.
		ProposalVoted { who: T::AccountId, proposal_id: u32 },
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
		/// No Proposal created by caller.
		NoProposalCreatedByCaller,
		/// Proposal already in voting period.
		ProposalAlreadyinVotingPeriod,
		/// Proposal not in voting period
		ProposalNotinVotingPeriod,
		/// Proposal Id storage must be empty
		ProposalIdStorageMustBeEmpty,
		/// No storage for this Proposal Id
		NoStorageForProposalId,
		/// Proposer cannot vote on their own proposal.
		ProposerCannotVote,
		/// Can't vote twice.
		CantVoteTwice,
		/// Arithmetic overflow.
		ArithmeticOverflow,
		/// Can't delegate to self.
		CantDelegateToSelf,
		/// Self delegation route detected.
		SelfDelegateRouteDetected,
		/// No storage for voter during delegation.
		NoStorageForVoterDuringDelegation,
		/// Can't delegate to anyone if already voted.
		CantDelegateToAnyoneIfAlreadyVoted,
	}

	/// Dispatchable for creating a new proposal.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A dispatchable for creating a proposal. This function requires a signed transaction.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn create_proposal(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::MaxStringLength>,
			start_timestamp: T::BlockNumber,
			end_timestamp: T::BlockNumber,
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
			let proposal_id = <LastProposalIndex<T>>::get().unwrap_or(0);
			let new_proposal_id =
				proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

			let proposal = Proposal {
				proposer: who.clone(),
				name,
				vote_count: 0,
				vote_start_timestamp: start_timestamp.into(),
				vote_end_timestamp: end_timestamp.into(),
			};

			match <Proposals<T>>::get(proposal_id) {
				Some(_) => return Err(Error::<T>::ProposalIdStorageMustBeEmpty.into()),
				None => {
					// Update storage for proposal
					<Proposals<T>>::insert(new_proposal_id, &proposal);

					// Update storage for proposal index
					<LastProposalIndex<T>>::put(new_proposal_id);

					// Emit an event.
					Self::deposit_event(Event::ProposalCreated {
						who,
						proposal_id: new_proposal_id,
					});

					// Return a successful DispatchResultWithPostInfo
					Ok(())
				},
			}
		}

		/// A dispatchable for cancelling a proposal. This function requires a signed transaction.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			ensure!(proposal_id > 0, Error::<T>::ZeroProposalId);

			match <Proposals<T>>::get(proposal_id) {
				Some(p) => {
					// ensure the proposal is not in voting period
					ensure!(
						Some(<frame_system::Pallet<T>>::block_number()) < p.vote_start_timestamp,
						Error::<T>::ProposalAlreadyinVotingPeriod
					);

					// Remove the proposal from storage.
					<Proposals<T>>::remove(proposal_id);

					// Emit an event.
					Self::deposit_event(Event::ProposalCancelled { who, proposal_id });
				},
				None => {
					return Err(Error::<T>::NoStorageForProposalId.into());
				},
			}

			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// A dispatchable for voting on a proposal. This function requires a signed transaction.
		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn vote(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// ensure the proposal is valid
			ensure!(proposal_id > 0, Error::<T>::ZeroProposalId);

			match <Voters<T>>::get(&who) {
				None => {
					match <Proposals<T>>::get(proposal_id) {
						Some(mut p) => {
							// ensure the proposal is in voting period
							ensure!(
								Some(<frame_system::Pallet<T>>::block_number())
									>= p.vote_start_timestamp && Some(
									<frame_system::Pallet<T>>::block_number()
								) <= p.vote_end_timestamp,
								Error::<T>::ProposalNotinVotingPeriod
							);

							// ensure that the voter is not the proposer
							ensure!(who != p.proposer, Error::<T>::ProposerCannotVote);

							// Update storage for voter
							let new_voter = Voter {
								weight: 1,
								voted: true,
								proposal: proposal_id,
								delegate: None,
							};
							<Voters<T>>::insert(&who, &new_voter);

							// Update storage for proposal with new vote count
							let new_vote_count = p
								.vote_count
								.checked_add(1)
								.ok_or(Error::<T>::ArithmeticOverflow)?;
							p.vote_count = new_vote_count;
							<Proposals<T>>::insert(proposal_id, &p);

							// Emit an event.
							Self::deposit_event(Event::ProposalVoted { who, proposal_id });

							Ok(())
						},
						None => {
							return Err(Error::<T>::NoStorageForProposalId.into());
						},
					}
				},
				Some(_) => return Err(Error::<T>::CantVoteTwice.into()),
			}
		}

		/// A dispatchable for delegating a vote. This function requires a signed transaction.
		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn delegate_vote(origin: OriginFor<T>, to: T::AccountId) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// ensure the `to` account is not the same as the `signer`
			ensure!(who != to, Error::<T>::CantDelegateToSelf);

			// the caller should have the some storage as there is default trait implemented.
			ensure!(<Voters<T>>::contains_key(&who), Error::<T>::NoStorageForVoterDuringDelegation);

			let mut to_temp: T::AccountId = to.clone();
			// ensure there is no self-delegation route.
			while let Some(v2) = <Voters<T>>::get(&to_temp) {
				if v2.delegate != None {
					to_temp = v2.delegate.unwrap();

					// ensure the subsequent delegate is not the same as the `signer`
					ensure!(who != to_temp, Error::<T>::SelfDelegateRouteDetected);
				}
			}

			if let Some(v) = <Voters<T>>::get(&who) {
				// ensure the `caller` account has not voted
				ensure!(!v.voted, Error::<T>::AlreadyVoted);

				if let Some(d) = <Voters<T>>::get(&to) {
					// if the delegate already voted, directly add to the number of votes for the proposal
					if d.voted {
						// add the vote to the proposal
						if let Some(mut p) = <Proposals<T>>::get(d.proposal) {
							// ensure the proposal is in voting period
							ensure!(
								Some(<frame_system::Pallet<T>>::block_number())
									>= p.vote_start_timestamp && Some(
									<frame_system::Pallet<T>>::block_number()
								) <= p.vote_end_timestamp,
								Error::<T>::ProposalNotinVotingPeriod
							);

							// Update storage for proposal with new vote count
							let new_vote_count = p
								.vote_count
								.checked_add(d.weight)
								.ok_or(Error::<T>::ArithmeticOverflow)?;
							p.vote_count = new_vote_count;
							<Proposals<T>>::insert(d.proposal, &p);
						}
					}
					// if the delegate has not voted, add to the weight of the delegate
					else {
						let new_weight =
							d.weight.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

						// Update storage for delegate
						let new_delegate = Voter { weight: new_weight, ..d };
						<Voters<T>>::insert(&to, &new_delegate);
					}
				}
			}

			Ok(())
		}
	}
}
