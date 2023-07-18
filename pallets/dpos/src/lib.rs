//! # DPoS Pallet
//!
//! A pallet for DPoS consensus algorithm implementation inspired from EOS blockchain.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Storage`]
//! - [`Event`]
//!
//! ## Overview
//!
//! The DPoS pallet is a consensus algorithm that is used to elect validators that can author blocks.
//! The validators are elected based on the votes they receive from the token holders.
//!
//! The token holders can vote for the validators they want to elect and the validators with the most votes are elected.
//!
//! Following are the components of the DPoS pallet:
//!
//! ### Staking
//!
//! Anyone can stake their tokens (greater than min. staking amount set) to vote for a validator.
//!
//! ### Voting
//!
//! Any staked token holder can vote for a validator. The vote is proportional to the amount of tokens staked.
//!
//! Any staked token holder can delegate their voting power to another account so that they can vote for the
//! selected validators (30).
//!
//! There is also a factor called "vote decay" which is used to reduce the voting power of a token holder over time.
//!
//! The voters would also get the rewards earned from the treasury pool if they vote for the elected validators.
//!
//! ### Block Production & Finalization
//!
//! The top 21 validators with the most votes are elected as validators that can author blocks.
//! The remaining validators (50) are on standby and can replace a validator if they have more votes.
//! The validators (active + standby) are renominated every 14_400 blocks (~ ONE_DAY).
//!
//! The standby Block Producers (BPs) have to signal that they are alive by sending a heartbeat every few (say 10) blocks.
//!
//! In a cycle, The active validators are rewarded with tokens for every block they produce and they are punished for every block they miss.
//! For standby validators, they are rewarded based on their consistent heartbeat signals and they lose their earned rewards for the times
//! they missed sending their heartbeat.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `stake`
//! - `unstake`
//! - `heartbeat`
//! - `reward`
//! - `register_as_bp`
//! - `deregister_as_bp`
//! - `vote`
//! - `delegate_vote`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Get, LockIdentifier, LockableCurrency},
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

	const ID1: LockIdentifier = *b"DPoS____";

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::MyCurrency as Currency<AccountOf<T>>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + TypeInfo {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// MyCurrency type for this pallet. Here, we could have used `Currency` trait.
		/// But, we need to use `set_lock` function which is not available in `Currency` trait.
		/// That's why `LockableCurrency` trait is used which itself inherits `Currency` trait.
		type MyCurrency: LockableCurrency<Self::AccountId>;

		/// The minimum amount of tokens that can be staked.
		#[pallet::constant]
		type MinStakeAmount: Get<BalanceOf<Self>>;

		/// The no. of validators that can be voted for by a single account.
		/// NOTE: Here, `u8` was supposed to be used but it was giving error related to trait bounds
		/// And hence, `u32` is used.
		#[pallet::constant]
		type MaxVotesPerAccount: Get<u32>;

		/// No. of validators that can author blocks i.e. Active Validators
		#[pallet::constant]
		type ActiveValidatorsCount: Get<u8>;

		/// No. of validators that are on standby i.e. Standby Validators
		#[pallet::constant]
		type StandbyValidatorsCount: Get<u16>;

		/// Every no. of blocks, the validators are ranked via latest ranking.
		#[pallet::constant]
		type RankingDuration: Get<u32>;

		/// Heartbeat duration in blocks
		#[pallet::constant]
		type HeartbeatDuration: Get<u32>;
	}

	#[derive(
		Decode, Encode, TypeInfo, Clone, PartialEq, Eq, Default, RuntimeDebug, MaxEncodedLen,
	)]
	// #[scale_info(skip_type_params(T))]
	pub struct VotingInfo<T: Config> {
		delegate_to: T::AccountId,
		cycle_no: u32,
		votes: BoundedVec<u8, T::MaxVotesPerAccount>,
	}

	/// Voting status of an account
	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type Voting<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, VotingInfo<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Staked { user: T::AccountId, stake_amt: BalanceOf<T> },
		Unstaked { user: T::AccountId, unstake_amt: BalanceOf<T> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Zero stake amount.
		ZeroStakeAmount,
		/// Zero Unstake amount.
		ZeroUnstakeAmount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Stake amount of tokens
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::stake())]
		pub fn stake(origin: OriginFor<T>, something: u32, amount: BalanceOf<T>) -> DispatchResult {
			/* 			// Check that the extrinsic was signed and get the signer.
					   // This function will return an error if the extrinsic is not signed.
					   // https://docs.substrate.io/main-docs/build/origins/
					   let who = ensure_signed(origin)?;

					   // Update storage.
					   <Something<T>>::put(something);

					   // Emit an event.
					   Self::deposit_event(Event::SomethingStored { something, who });
					   // Return a successful DispatchResultWithPostInfo
			*/
			Ok(())
		}

		/// Unstake amount of tokens
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::unstake())]
		pub fn unstake(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			/* 			let _who = ensure_signed(origin)?;

					   // Read a value from storage.
					   match <Something<T>>::get() {
						   // Return an error if the value has not been set.
						   None => return Err(Error::<T>::NoneValue.into()),
						   Some(old) => {
							   // Increment the value read from storage; will error in the event of overflow.
							   let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
							   // Update the value in storage with the incremented result.
							   <Something<T>>::put(new);
							   Ok(())
						   },
					   }
			*/

			Ok(())
		}
	}
}
