//! # Bank Pallet
//!
//! A simple pallet demonstrating the usage of `ReservableCurrency`,
//! `NamedReservableCurrency`, `Lockable` traits.
//!
//! - [`pallet::Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! Anyone can open FD (Fixed Deposit) by reserving some amount of currency.
//!
//! During the FD period, the reserved amount cannot be used. If the FD is closed before 100 blocks,
//! then the reserved amount is returned to the user without any interest.
//!
//! But, if the FD is closed after 100 blocks, then the reserved amount is returned to the user with
//! some interest. The interest is stored & set by the root origin.
//!
//! The interest comes from a treasury 💎 account which is funded by the root origin.
//!
//! NOTE: The runtime must include the `Balances` pallet to handle the accounts and balances for your chain.
//!
//!
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! TODO: set interest rate, blocks limit in the runtime's next immediate
//! block after the pallet deployment.
//!
//! - `set_fd_interest_rate`
//! - `set_fd_blocks_limit`
//! - `open_fd`
//! - `close_fd`
//! - `lock_for_dao`
//! - `unlock`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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

	use frame_support::traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons};
	use frame_support::traits::{LockIdentifier, NamedReservableCurrency};

	const ID1: LockIdentifier = *b"Bank    ";

	type AccountOf<T> = <T as frame_system::Config>::AccountId; // optional
	type BalanceOf<T> = <<T as Config>::MyCurrency as Currency<AccountOf<T>>>::Balance;

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
		type MyCurrency: ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>
			+ NamedReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type MinFDValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxFDValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinLockValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxLockValue: Get<BalanceOf<Self>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn fd_interest)]
	pub type FDInterest<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
		/// FD Opened
		FDOpened {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// FD Closed
		FDClosed {
			user: T::AccountId, // can also use `AccountOf<T>`
			block: T::BlockNumber,
		},

		/// FD Interest Set
		FDInterestSet { interest: BalanceOf<T> },

		/// Locked for DAO
		LockedForDAO {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// Unlocked
		Unlocked {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
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
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		fn get_frt_balances(
			caller: &T::AccountId,
		) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
			let f = T::MyCurrency::free_balance(&caller);
			let r = T::MyCurrency::reserved_balance(&caller);
			let t = T::MyCurrency::total_balance(&caller);

			Ok((f, r, t))
		}

		// fn validate_amount(amount: BalanceOf<T>) -> Result<(), DispatchError> {
		// 	if amount == T::MinStakedValue::get() || amount > T::MaxStakedValue::get() {
		// 		return Err(Error::<T>::EitherMinMaxStakeAmountParsed.into());
		// 	}

		// 	Ok(())
		// }
	}
}
