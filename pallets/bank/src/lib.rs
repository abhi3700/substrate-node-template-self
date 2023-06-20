//! # Bank Pallet
//!
//! A simple pallet demonstrating the usage of `ReservableCurrency`,
//! `NamedReservableCurrency` & `LockableCurrency` trait.
//!
//! - [`pallet::Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `set_balance`
//! - `update_balance` if a user's nonce is at least 2 more than the previous.
//! - `reserve`
//! - `unreserve`
//! - `lock`
//! - `unlock`

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::LockIdentifier;
use frame_support::traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons};

pub use pallet::*;

const ID1: LockIdentifier = *b"Staking ";

type _AccountOf<T> = <T as frame_system::Config>::AccountId; // optional
type BalanceOf<T> =
	<<T as Config>::MyCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_support::dispatch::DispatchError;
	use frame_support::log;
	use frame_support::pallet_prelude::ValueQuery;
	use frame_support::sp_runtime::DispatchError;
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;
	// use sp_runtime::traits::Zero; // TODO: `$ cargo add sp_runtime --no-default-features`

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// MyCurrency type for this pallet. Here, we could have used `Currency` trait.
		/// But, we need to use `reserved_balance` function which is not available in `Currency` trait.
		/// That's why `ReservableCurrency` trait is used.
		type MyCurrency: ReservableCurrency<Self::AccountId> + LockableCurrency<Self::AccountId>;

		#[pallet::constant]
		type MinStakedValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxStakedValue: Get<BalanceOf<Self>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn some_staking)]
	// NOTE: can also use `AccountOf<T>` instead of `T::AccountId` here.
	pub type SomeStaking<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>, BalanceOf<T>),
		// ValuQuery, // optional
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Staked.
		Staked {
			caller: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// Unstaked
		Unstaked {
			caller: T::AccountId, // can also use `AccountOf<T>`
			old_staked: Option<BalanceOf<T>>,
			new_staked: Option<BalanceOf<T>>,
			current_block: T::BlockNumber,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Zero Stake Amount
		ZeroStakeAmount,
		/// Either min/max stake amount parsed
		EitherMinMaxStakeAmountParsed,
		/// Already Max. Staked
		AlreadyMaxStaked,
		/// Insufficient for Unstake
		InsufficientForUnstake,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Stake
		///
		/// During each time, update the free, reserve, total balances
		/// TEST:
		///
		/// If Alice stakes
		/// ```
		/// - check for --0-->{f: 100, s: 0} ❌
		/// - check for --u128::Max-->{f: 100, s: 0} ❌
		/// - check for --10-->{f: 100, s: 0} ✅
		/// - check for --5-->{f: 90, s: 10} ✅
		/// - check for --10-->{f: 90, s: 10} ✅
		/// - check for --15-->{f:90, s:10} ✅
		/// ```
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn stake(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// TODO: check if this validation is required as I didn't found any such validation in `Staking` pallet or
			// may be they already have it inbuilt in `set_lock`/`remove_lock` function
			let _ = Self::validate_amount(amount);

			let (f, r, t) = Self::get_frt_balances(&caller)?;

			// lock the amount as staked
			T::MyCurrency::set_lock(ID1, &caller, amount, WithdrawReasons::RESERVE);

			// update the new staked amount
			if let Some((_, _, _, _)) = <SomeStaking<T>>::get(&caller) {
				// Get the old staked amount.
				let (_, _, old_s, _) = <SomeStaking<T>>::get(&caller).unwrap();

				// Calculate the new staked amount.
				let new_staked_amount = old_s + amount;

				// Ensure that the new staked amount is within the maximum limit.
				ensure!(new_staked_amount < T::MaxStakedValue::get(), "MaxStakedValue reached");

				// Update the storage with the new staked amount.
				<SomeStaking<T>>::insert(&caller, (f, r, new_staked_amount, t));
			} else {
				// If there was no previous staked amount, set the new staked amount to the given amount.
				<SomeStaking<T>>::insert(&caller, (f, r, amount, t));
			}

			// Emit an event.
			Self::deposit_event(Event::Staked {
				caller,
				amount,
				block: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}

		/// Unstake
		/// During each time, update the free, reserve, total balances
		///
		/// TEST:
		///
		/// If Alice unstakes with {f:50, s:50} condition,
		/// ```
		/// - check for --0-->{f:50, s:50} ❌
		/// - check for --u128::Max-->{f:90, s:10} ❌
		/// - check for --10-->{f:50, s:50} ✅
		/// - check for --51-->{f:50, s:50} ❌
		/// - check for --10-->{f: 90, s: 10} ✅
		/// - check for --15-->{f:90, s:10} ✅
		/// ```
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn unstake(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let _ = Self::validate_amount(amount);

			// remove the lock
			T::MyCurrency::remove_lock(ID1, &caller);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_frt_balances(
			caller: &T::AccountId,
		) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
			let f = T::MyCurrency::free_balance(&caller);
			let r = T::MyCurrency::reserved_balance(&caller);
			let t = T::MyCurrency::total_balance(&caller);

			Ok((f, r, t))
		}

		fn validate_amount(amount: BalanceOf<T>) -> Result<(), DispatchError> {
			if amount == T::MinStakedValue::get() || amount > T::MaxStakedValue::get() {
				return Err(Error::<T>::EitherMinMaxStakeAmountParsed.into());
			}

			Ok(())
		}
	}
}
