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
//! - `unlock_for_dao`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// The log target.
const TARGET: &'static str = "pallet_bank::close_fd";

#[frame_support::pallet]
#[allow(unused)]	// no warning shown
pub mod pallet {
	
	use super::*;
	use frame_support::log;
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;
	// `$ cargo add sp-runtime -p pallet-bank --no-default-features` at the node-template repo root.
	use sp_runtime::traits::{CheckedDiv, CheckedMul, CheckedSub, Zero};

	use frame_support::traits::{Currency, LockableCurrency, ReservableCurrency, LockIdentifier, NamedReservableCurrency, WithdrawReasons, ExistenceRequirement::{AllowDeath}};

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
	// in percentage i.e. 0.5% = 0.005 => represented as 1e5 (scaling_factor) => 500
	// NOTE: We can put this scaling factor as high as possible i.e. 1e18,
	// but then during division it would lose the precision. Hence, choose as small as possible.
	// Hence, keep the scaling_factor as low as possible.
	// make sure during arithmetic, you divide by 1e5
	//
	// E.g. If the interest rate is 0.005%, then the interest is 0.005e5 = 500
	// If the interest rate is 10%, then the interest is 10e5 = 1_000_000
	//
	// (u32, u32) tuple represents (interest, scaling_factor)
	// NOTE: type of `interest` has been kept as `u32` covering the whole range of possible percentages as high as 10%
	// & as low as 0.00001%
	// It is recommended to set the scaling factor 1e5 in general.
	pub type FDInterest<T: Config> = StorageValue<_, (u32, u32)>;

	#[pallet::storage]
	#[pallet::getter(fn fd_block_duration)]
	// Block limit for FD closure.
	pub type FDBlockDuration<T: Config> = StorageValue<_, u16>;

	#[pallet::storage]
	#[pallet::getter(fn treasury)]
	// Treasury account.
	pub type Treasury<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn fd_user_ids)]
	// Next FD User IDs for each user.
	pub type FDUserIds<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u16, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn fd_vault)]
	// NOTE: can also use `AccountOf<T>` instead of `T::AccountId` here.
	// user -> id -> (amount, block_number)
	pub type FDVaults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		u16,
		(BalanceOf<T>, T::BlockNumber),
		// ValuQuery, // optional
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Set FD Interest Rate & Scaling Factor
		FDInterestSet { interest: u32, scaling_factor: u32 },

		/// Set block duration
		FDBlockDurationSet { block_duration: u16 },

		/// Treasury account set
		TreasurySet { block_num: T::BlockNumber },

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

		/// Locked for DAO
		LockedForDAO {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// Unlocked for DAO
		UnlockedForDAO {
			user: T::AccountId, // can also use `AccountOf<T>`
			block: T::BlockNumber,
		},

	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Zero Interest Rate
		ZeroInterestRate,
		/// Interest Not Set
		InterestNotSet,
		/// Zero FD Block Duration
		ZeroFDBlockDuration,
		/// FD Block Duration Not Set
		FDBlockDurationNotSet,
		/// Zero Amount When Opening FD
		ZeroAmountWhenOpeningFD,
		/// Insufficient Free Balance When Opening FD
		InsufficientFreeBalanceWhenOpeningFD,
		/// FD Already Exists With Id When Opening FD
		FDAlreadyExistsWithIdWhenOpeningFD,
		/// FD Does Not Exist With Id When Closing FD
		FDNotExistsWithIdWhenClosingFD,
		/// Treasury Not Set
		TreasuryNotSet,
		/// Arithmetic Underflow
		ArithmeticUnderflow,
		/// Lock Amount is Less Than Min Lock Amount
		LockAmountIsLessThanMinLockAmount,
		/// Lock Amount is Greater Than Max Lock Amount
		LockAmountExceedsMaxLockAmount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> 
	// NOTE: Here, `where` clause is required for converting from `BlockNumber` to `Balance`
	// automatically suggested from compiler.
	where
			<<T as pallet::Config>::MyCurrency as Currency<
				<T as frame_system::Config>::AccountId,
			>>::Balance: From<<T as frame_system::Config>::BlockNumber>, {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_fd_interest_rate(
			origin: OriginFor<T>,
			interest: u32,
			scaling_factor: u32,
		) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// ensure positive interest
			ensure!(interest > 0, Error::<T>::ZeroInterestRate);

			// ensure scaling factor is not zero
			ensure!(scaling_factor > 0, Error::<T>::ZeroInterestRate);

			// set the interest rate
			FDInterest::<T>::put((interest, scaling_factor));

			// emit the event
			Self::deposit_event(Event::FDInterestSet { interest, scaling_factor });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_fd_blocks_duration(origin: OriginFor<T>, block_duration: u16) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// ensure the block limit is not zero
			ensure!(block_duration > 0, Error::<T>::ZeroFDBlockDuration);

			// set the block limit
			FDBlockDuration::<T>::put(block_duration);

			// emit the event
			Self::deposit_event(Event::FDBlockDurationSet { block_duration });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_treasury(origin: OriginFor<T>, treasury: T::AccountId) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// set the treasury
			Treasury::<T>::put(treasury);

			// emit the event
			Self::deposit_event(Event::TreasurySet {
				block_num: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn open_fd(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure the treasury is set
			ensure!(Treasury::<T>::get().is_some(), Error::<T>::TreasuryNotSet);

			// ensure the interest details set
			ensure!(<FDInterest<T>>::exists(), Error::<T>::InterestNotSet);

			// ensure the amount is not zero
			ensure!(amount > Zero::zero(), Error::<T>::ZeroAmountWhenOpeningFD);

			// get the next fd id for the user
			let next_fd_id = Self::get_next_fd_id(&user);

			// ensure there is no FD with the id received
			ensure!(
				!FDVaults::<T>::contains_key(&user, next_fd_id),
				Error::<T>::FDAlreadyExistsWithIdWhenOpeningFD
			);

			// NOTE: inherently checked for sufficient free balance
			// reserve the token as supposed to be deducted from free_balance.
			T::MyCurrency::reserve(&user, amount)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// store the FD details in the storage for the user
			FDVaults::<T>::insert(&user, next_fd_id, (amount, current_block_number));

			// update the next fd id for the user
			FDUserIds::<T>::insert(&user, next_fd_id + 1);

			// emit the event
			Self::deposit_event(Event::FDOpened { user, amount, block: current_block_number });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn close_fd(origin: OriginFor<T>, id: u16) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure there is a treasury account set & get that if exists
			let treasury = <Treasury<T>>::get().ok_or(Error::<T>::TreasuryNotSet)?;

			// get the FD details
			let (amount, old_block_num) =
				FDVaults::<T>::get(&user, id).ok_or(Error::<T>::FDNotExistsWithIdWhenClosingFD)?;

			// get the interest if exists
			let (interest_rate, scaling_factor) =
				FDInterest::<T>::get().ok_or(Error::<T>::InterestNotSet)?;

			// get the block duration if exists
			let block_duration =
				FDBlockDuration::<T>::get().ok_or(Error::<T>::FDBlockDurationNotSet)?;

			// get the current block number
			let current_block_num = <frame_system::Pallet<T>>::block_number();

			// get the block difference if any
			let block_difference = <frame_system::Pallet<T>>::block_number()
				.checked_sub(&old_block_num)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;

			// TODO: Add FD expiry date as param in storage & corresponding logic

			// if the FD is open for min. duration i.e. block_duration, then calculate the interest
			// & transfer the amount + interest from the treasury account to the caller;
			// else transfer the amount only from the treasury account to the caller
			if block_difference > block_duration.into() {
				// M-1: calculate the interest directly
				// let interest = amount
				// 	.checked_mul(&interest_rate.into())
				// 	.and_then(|v| v.checked_mul(&block_difference.into()))
				// 	.and_then(|v| v.checked_div(&block_duration.into()))
				// 	.and_then(|v| v.checked_div(&scaling_factor.into()))
				// 	.ok_or("Interest calculation failed")?;

				// M-2: calculate the interest indirectly using Nr/Dr approach
				// TODO: wrap this code snippet in a function `get_interest` & use it here.
				let numerator = amount
					.checked_mul(&interest_rate.into())
					.and_then(|v| v.checked_mul(&block_difference.into()))
					.ok_or("Interest Numerator calculation failed")?;
				let denominator = block_duration
					.checked_mul(scaling_factor as u16)
					.ok_or("Interest Denominator calculation failed")?;
				let interest = numerator
					.checked_div(&denominator.into())
					.ok_or("Interest calculation failed")?;

				// TODO: debug the value using both the approaches above.
				log::info!(target: TARGET, "Interest: {:?}", interest);

				// transfer the interest from the treasury account to the user
				let _ = T::MyCurrency::transfer(
					&treasury,
					&user,
					interest,
					AllowDeath,
				);
			}

			// unreserve the amount from the user
			T::MyCurrency::unreserve(&user, amount);

			// remove the FD details from the storage for the user
			<FDVaults<T>>::remove(&user, id);

			// emit the event
			Self::deposit_event(Event::FDClosed { user, block: current_block_num });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn lock_for_dao(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure that the amount is at least min. lock amount
			ensure!(amount >= T::MinLockValue::get(), Error::<T>::LockAmountIsLessThanMinLockAmount);

			// ensure that the amount is < max lock amount
			ensure!(amount <= T::MaxLockValue::get(), Error::<T>::LockAmountExceedsMaxLockAmount);

			// lock amount
			T::MyCurrency::set_lock(ID1, &user, amount, WithdrawReasons::all());

			// emit the event
			Self::deposit_event(Event::LockedForDAO { 
				user, amount, 
				block: <frame_system::Pallet<T>>::block_number() });

			Ok(())}

		#[pallet::call_index(6)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn unlock_for_dao(origin: OriginFor<T>) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// unlock amount
			T::MyCurrency::remove_lock(ID1, &user);

			// emit the event
			Self::deposit_event(Event::UnlockedForDAO { 
				user,
				block: <frame_system::Pallet<T>>::block_number() });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// Get the next available FD id for the user
		fn get_next_fd_id(user: &T::AccountId) -> u16 {
			let next_fd_id = FDUserIds::<T>::get(user) + 1;
			next_fd_id
		}

		// fn get_interest_amt(
		// 	amount: BalanceOf<T>,
		// 	scaling_factor: u16,
		// 	block_duration: u16,
		// 	block_difference: T::BlockNumber,
		// ) -> BalanceOf<T> {
		// 	let interest = ((amount as u128)
		// 		.checked_mul((scaling_factor as u128).checked_mul(block_difference as u128)))
		// 	.checked_div((block_duration as u128))
		// 	.unwrap();

		// 	interest
		// }

		// 	Ok(())
		// }
	}
}
